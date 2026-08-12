/** Defines the shared job-scoped Company Research rendering contract. */

import type { ReactNode } from "react";

export interface CompanyResearchTarget {
  companyName: string;
  jobHash?: string | null;
}

export interface CompanyResearchPanelProps {
  companyName: string;
  jobHash?: string | null;
  onClose?: () => void;
}

export type RenderCompanyResearch = (
  props: CompanyResearchPanelProps,
) => ReactNode;
