export interface JobSourceGuidance {
  label: string;
  description: string;
  review?: {
    title: string;
    description: string;
    ariaLabel: string;
    tone?: "neutral" | "warning";
  };
}

const EMPLOYER_SOURCE_DESCRIPTION =
  "Closer to the employer source. Still verify before tailoring.";
const JOB_BOARD_SOURCE_DESCRIPTION =
  "Job-board source. Verify on the employer page before tailoring.";
const COMMUNITY_SOURCE_DESCRIPTION =
  "Community job source. Verify on the employer page before tailoring.";
const UNKNOWN_SOURCE_GUIDANCE: JobSourceGuidance = {
  label: "Source not shown",
  description:
    "No source was recorded for this posting. Open the original job page before tailoring.",
  review: {
    title: "Source not shown",
    description:
      "No source was recorded for this posting. Open the original job page before tailoring.",
    ariaLabel: "Source not shown, open original job page before tailoring",
    tone: "warning",
  },
};

const JOB_BOARD_SOURCE_REVIEW = {
  title: "Verify employer page",
  description:
    "Job-board listings can lag behind employer pages. Open the original role before tailoring.",
  ariaLabel: "Verify employer page before tailoring",
};

const COMMUNITY_SOURCE_REVIEW = {
  title: "Verify official posting",
  description:
    "Community posts can be useful leads. Confirm the role on an employer or hiring page before tailoring.",
  ariaLabel: "Verify official posting before tailoring",
};

const CONNECTED_SOURCE_REVIEW = {
  title: "Verify connected source",
  description:
    "This came from a connected local source. Check the employer page before tailoring.",
  ariaLabel: "Verify connected source before tailoring",
};

const SAVED_LINK_SOURCE_REVIEW = {
  title: "Check saved link",
  description:
    "Saved from a link you chose. Confirm the original posting is still open before tailoring.",
  ariaLabel: "Check saved link before tailoring",
};

const SAVED_LINK_SOURCE_GUIDANCE: JobSourceGuidance = {
  label: "Saved by you",
  description: "Saved from a link you chose. Check the original posting before tailoring.",
  review: SAVED_LINK_SOURCE_REVIEW,
};

const SAMPLE_SOURCE_REVIEW = {
  title: "Replace sample job",
  description:
    "Sample jobs are for local review. Use current postings before tailoring.",
  ariaLabel: "Replace sample job before tailoring",
};

const CUSTOM_SOURCE_REVIEW = {
  title: "Check source before tailoring",
  description:
    "JobSentinel only has the source label from this posting. Verify the role on an employer page before tailoring.",
  ariaLabel: "Check source before tailoring",
};

const SOURCE_GUIDANCE_BY_KEY: Record<string, JobSourceGuidance> = {
  greenhouse: {
    label: "Greenhouse hiring page",
    description: EMPLOYER_SOURCE_DESCRIPTION,
  },
  lever: {
    label: "Lever hiring page",
    description: EMPLOYER_SOURCE_DESCRIPTION,
  },
  ashby: {
    label: "Ashby hiring page",
    description: EMPLOYER_SOURCE_DESCRIPTION,
  },
  smartrecruiters: {
    label: "SmartRecruiters hiring page",
    description: EMPLOYER_SOURCE_DESCRIPTION,
  },
  usajobs: {
    label: "USAJobs official source",
    description: EMPLOYER_SOURCE_DESCRIPTION,
  },
  yc_startup: {
    label: "Y Combinator hiring page",
    description: EMPLOYER_SOURCE_DESCRIPTION,
  },
  linkedin: {
    label: "LinkedIn job board",
    description: JOB_BOARD_SOURCE_DESCRIPTION,
    review: JOB_BOARD_SOURCE_REVIEW,
  },
  indeed: {
    label: "Indeed job board",
    description: JOB_BOARD_SOURCE_DESCRIPTION,
    review: JOB_BOARD_SOURCE_REVIEW,
  },
  glassdoor: {
    label: "Glassdoor job board",
    description: JOB_BOARD_SOURCE_DESCRIPTION,
    review: JOB_BOARD_SOURCE_REVIEW,
  },
  ziprecruiter: {
    label: "ZipRecruiter job board",
    description: JOB_BOARD_SOURCE_DESCRIPTION,
    review: JOB_BOARD_SOURCE_REVIEW,
  },
  dice: {
    label: "Dice job board",
    description: JOB_BOARD_SOURCE_DESCRIPTION,
    review: JOB_BOARD_SOURCE_REVIEW,
  },
  simplyhired: {
    label: "SimplyHired job board",
    description: JOB_BOARD_SOURCE_DESCRIPTION,
    review: JOB_BOARD_SOURCE_REVIEW,
  },
  remoteok: {
    label: "Remote OK job board",
    description: JOB_BOARD_SOURCE_DESCRIPTION,
    review: JOB_BOARD_SOURCE_REVIEW,
  },
  weworkremotely: {
    label: "We Work Remotely job board",
    description: JOB_BOARD_SOURCE_DESCRIPTION,
    review: JOB_BOARD_SOURCE_REVIEW,
  },
  monster: {
    label: "Monster job board",
    description: JOB_BOARD_SOURCE_DESCRIPTION,
    review: JOB_BOARD_SOURCE_REVIEW,
  },
  careerbuilder: {
    label: "CareerBuilder job board",
    description: JOB_BOARD_SOURCE_DESCRIPTION,
    review: JOB_BOARD_SOURCE_REVIEW,
  },
  hn_hiring: {
    label: "Startup and tech job posts",
    description: COMMUNITY_SOURCE_DESCRIPTION,
    review: COMMUNITY_SOURCE_REVIEW,
  },
  jobswithgpt: {
    label: "Connected job source",
    description: "From a connected local source. Verify on the employer page before tailoring.",
    review: CONNECTED_SOURCE_REVIEW,
  },
  import: SAVED_LINK_SOURCE_GUIDANCE,
  user_source_actions: SAVED_LINK_SOURCE_GUIDANCE,
  manual: SAVED_LINK_SOURCE_GUIDANCE,
  manual_import: SAVED_LINK_SOURCE_GUIDANCE,
  builtin: {
    label: "Sample job",
    description: "Sample data for local review. Replace it with current postings before tailoring.",
    review: SAMPLE_SOURCE_REVIEW,
  },
  test: {
    label: "Sample job",
    description: "Sample data for local review. Replace it with current postings before tailoring.",
    review: SAMPLE_SOURCE_REVIEW,
  },
};

function normalizeJobSourceKey(source: string) {
  return source
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function titleCaseSource(source: string) {
  const cleaned = source.trim().replace(/[_-]+/g, " ").replace(/\s+/g, " ");
  if (!cleaned) {
    return "Unknown source";
  }

  return cleaned
    .split(" ")
    .map((part) => {
      const lower = part.toLowerCase();
      if (lower === "usa") return "USA";
      if (lower === "ok") return "OK";
      return lower.charAt(0).toUpperCase() + lower.slice(1);
    })
    .join(" ");
}

export function getJobSourceGuidance(source: string): JobSourceGuidance {
  const key = normalizeJobSourceKey(source);
  if (!key) {
    return UNKNOWN_SOURCE_GUIDANCE;
  }

  return SOURCE_GUIDANCE_BY_KEY[key] ?? {
    label: titleCaseSource(source),
    description:
      "Source label from this posting. Verify on the employer page before tailoring.",
    review: CUSTOM_SOURCE_REVIEW,
  };
}

export function formatJobSourceLabel(source: string) {
  return getJobSourceGuidance(source).label;
}
