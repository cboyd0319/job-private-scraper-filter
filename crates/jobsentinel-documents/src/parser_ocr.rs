use super::ResumeParser;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub(super) fn check_tesseract_available() -> bool {
    use std::process::Command;

    let Ok(tesseract_path) = resolve_ocr_tool(OcrTool::Tesseract) else {
        return false;
    };

    Command::new(tesseract_path)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

const fn tesseract_env_var() -> &'static str {
    "JOBSENTINEL_TESSERACT_PATH"
}

const fn pdftoppm_env_var() -> &'static str {
    "JOBSENTINEL_PDFTOPPM_PATH"
}

pub(super) fn ocr_pdf(parser: &ResumeParser, file_path: &Path) -> Result<String> {
    use std::process::Command;

    let pdftoppm_path =
        resolve_ocr_tool(OcrTool::PdfToPpm).context("pdftoppm is not available for OCR support")?;
    let tesseract_path = resolve_ocr_tool(OcrTool::Tesseract)
        .context("Tesseract OCR is not available for OCR support")?;

    let temp_dir = tempfile::Builder::new()
        .prefix("jobsentinel_ocr_")
        .tempdir()
        .context("Failed to create temp directory for OCR")?;
    let output_prefix = temp_dir.path().join("page");
    let pdftoppm_result = Command::new(&pdftoppm_path)
        .arg("-png")
        .arg("-r")
        .arg("300")
        .arg(file_path)
        .arg(&output_prefix)
        .output();

    let mut image_paths: Vec<PathBuf> = match pdftoppm_result {
        Ok(output) if output.status.success() => std::fs::read_dir(temp_dir.path())?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| validated_ocr_image_path(path, temp_dir.path()))
            .collect(),
        _ => {
            return Err(anyhow::anyhow!(
                "pdftoppm not available. Install poppler-utils for OCR support."
            ));
        }
    };

    if image_paths.is_empty() {
        return Err(anyhow::anyhow!("No images extracted from PDF"));
    }

    image_paths.sort();

    let mut full_text = String::new();
    for image_path in &image_paths {
        let output = Command::new(&tesseract_path)
            .arg(image_path)
            .arg("stdout")
            .arg("-l")
            .arg("eng")
            .output()
            .context("Failed to run Tesseract OCR")?;

        if output.status.success() {
            let page_text = String::from_utf8_lossy(&output.stdout);
            if !full_text.is_empty() {
                full_text.push_str("\n\n--- Page Break ---\n\n");
            }
            full_text.push_str(&page_text);
        }
    }

    if full_text.is_empty() {
        return Err(anyhow::anyhow!("Tesseract OCR returned no text"));
    }

    Ok(parser.clean_text(&full_text))
}

fn validated_ocr_image_path(path: &Path, temp_dir: &Path) -> bool {
    if path
        .extension()
        .map(|extension| extension == "png")
        .unwrap_or(false)
    {
        if let Ok(canonical) = path.canonicalize() {
            if let Ok(canonical_temp) = temp_dir.canonicalize() {
                return canonical.starts_with(&canonical_temp) && canonical.is_file();
            }
        }
    }
    false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OcrTool {
    Tesseract,
    PdfToPpm,
}

impl OcrTool {
    fn env_var(self) -> &'static str {
        match self {
            Self::Tesseract => tesseract_env_var(),
            Self::PdfToPpm => pdftoppm_env_var(),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Tesseract => "Tesseract OCR",
            Self::PdfToPpm => "pdftoppm",
        }
    }

    const fn default_candidates(self) -> &'static [&'static str] {
        match self {
            Self::Tesseract => &[
                "/opt/homebrew/bin/tesseract",
                "/usr/local/bin/tesseract",
                "/usr/bin/tesseract",
                r"C:\Program Files\Tesseract-OCR\tesseract.exe",
                r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe",
            ],
            Self::PdfToPpm => &[
                "/opt/homebrew/bin/pdftoppm",
                "/usr/local/bin/pdftoppm",
                "/usr/bin/pdftoppm",
                r"C:\Program Files\poppler\Library\bin\pdftoppm.exe",
                r"C:\Program Files (x86)\poppler\Library\bin\pdftoppm.exe",
            ],
        }
    }

    const fn trusted_roots(self) -> &'static [&'static str] {
        match self {
            Self::Tesseract => &[
                "/opt/homebrew/bin",
                "/usr/local/bin",
                "/usr/bin",
                r"C:\Program Files\Tesseract-OCR",
                r"C:\Program Files (x86)\Tesseract-OCR",
            ],
            Self::PdfToPpm => &[
                "/opt/homebrew/bin",
                "/usr/local/bin",
                "/usr/bin",
                r"C:\Program Files\poppler\Library\bin",
                r"C:\Program Files (x86)\poppler\Library\bin",
            ],
        }
    }
}

fn resolve_ocr_tool(tool: OcrTool) -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(tool.env_var()).filter(|value| !value.is_empty()) {
        return validate_ocr_tool_path(tool, PathBuf::from(path));
    }

    for candidate in tool.default_candidates() {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return validate_ocr_tool_path(tool, path);
        }
    }

    Err(anyhow::anyhow!(
        "{} executable was not found in a trusted install location",
        tool.label()
    ))
}

pub(super) fn validate_ocr_tool_path(tool: OcrTool, path: PathBuf) -> Result<PathBuf> {
    validate_ocr_tool_path_against_roots(tool, path, tool.trusted_roots())
}

pub(super) fn validate_ocr_tool_path_against_roots<I, P>(
    tool: OcrTool,
    path: PathBuf,
    trusted_roots: I,
) -> Result<PathBuf>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    if !path.is_absolute() {
        return Err(anyhow::anyhow!(
            "{} path must be an absolute executable path",
            tool.label()
        ));
    }

    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("{} executable is not accessible", tool.label()))?;

    if !canonical_path.is_file() {
        return Err(anyhow::anyhow!(
            "{} executable path is not a regular file",
            tool.label()
        ));
    }

    let parent = path.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "{} path must be in a trusted install location",
            tool.label()
        )
    })?;
    let canonical_parent = parent
        .canonicalize()
        .with_context(|| format!("{} executable parent is not accessible", tool.label()))?;

    let is_trusted = trusted_roots
        .into_iter()
        .filter_map(|root| root.as_ref().canonicalize().ok())
        .any(|root| path_starts_with(&canonical_parent, &root));

    if !is_trusted {
        return Err(anyhow::anyhow!(
            "{} path must be in a trusted install location",
            tool.label()
        ));
    }

    Ok(canonical_path)
}

fn path_starts_with(path: &Path, root: &Path) -> bool {
    #[cfg(windows)]
    {
        let path_components = lowercase_path_components(path);
        let root_components = lowercase_path_components(root);
        path_components.starts_with(&root_components)
    }

    #[cfg(not(windows))]
    {
        path.starts_with(root)
    }
}

#[cfg(windows)]
fn lowercase_path_components(path: &Path) -> Vec<String> {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect()
}
