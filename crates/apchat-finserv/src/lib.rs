//! Financial services skills for APChat.
//!
//! Adapted from <https://github.com/anthropics/financial-services-plugins>
//! (Apache-2.0 licensed). Provides 42 embedded skills covering:
//!
//! - **Financial Analysis** (core): comps, DCF, LBO, 3-statement models, competitive analysis
//! - **Investment Banking**: CIMs, teasers, buyer lists, merger models, pitch decks
//! - **Equity Research**: earnings analysis, initiating coverage, sector overviews
//! - **Private Equity**: deal sourcing, due diligence, IC memos, portfolio monitoring
//! - **Wealth Management**: client reports, financial plans, portfolio rebalancing

use std::collections::HashMap;

/// All financial services skill names (for bulk removal when the feature is disabled).
pub const FSI_SKILL_NAMES: &[&str] = &[
    "fsi-3-statements",
    "fsi-buyer-list",
    "fsi-catalyst-calendar",
    "fsi-check-deck",
    "fsi-check-model",
    "fsi-cim-builder",
    "fsi-client-report",
    "fsi-client-review",
    "fsi-competitive-analysis",
    "fsi-comps-analysis",
    "fsi-datapack-builder",
    "fsi-dcf-model",
    "fsi-dd-checklist",
    "fsi-dd-meeting-prep",
    "fsi-deal-screening",
    "fsi-deal-sourcing",
    "fsi-deal-tracker",
    "fsi-earnings-analysis",
    "fsi-earnings-preview",
    "fsi-financial-plan",
    "fsi-ic-memo",
    "fsi-idea-generation",
    "fsi-initiating-coverage",
    "fsi-investment-proposal",
    "fsi-lbo-model",
    "fsi-merger-model",
    "fsi-model-update",
    "fsi-morning-note",
    "fsi-pitch-deck",
    "fsi-portfolio-monitoring",
    "fsi-portfolio-rebalance",
    "fsi-ppt-template-creator",
    "fsi-process-letter",
    "fsi-returns-analysis",
    "fsi-sector-overview",
    "fsi-skill-creator",
    "fsi-strip-profile",
    "fsi-tax-loss-harvesting",
    "fsi-teaser",
    "fsi-thesis-tracker",
    "fsi-unit-economics",
    "fsi-value-creation-plan",
];

/// Returns all financial services skills as a map of name → SKILL.md content.
pub fn get_financial_services_skills() -> HashMap<&'static str, &'static str> {
    let mut skills = HashMap::new();

    // Financial Analysis (core)
    skills.insert("fsi-3-statements", include_str!("../skills/fsi-3-statements.md"));
    skills.insert("fsi-check-deck", include_str!("../skills/fsi-check-deck.md"));
    skills.insert("fsi-check-model", include_str!("../skills/fsi-check-model.md"));
    skills.insert("fsi-competitive-analysis", include_str!("../skills/fsi-competitive-analysis.md"));
    skills.insert("fsi-comps-analysis", include_str!("../skills/fsi-comps-analysis.md"));
    skills.insert("fsi-dcf-model", include_str!("../skills/fsi-dcf-model.md"));
    skills.insert("fsi-lbo-model", include_str!("../skills/fsi-lbo-model.md"));
    skills.insert("fsi-ppt-template-creator", include_str!("../skills/fsi-ppt-template-creator.md"));
    skills.insert("fsi-skill-creator", include_str!("../skills/fsi-skill-creator.md"));

    // Investment Banking
    skills.insert("fsi-buyer-list", include_str!("../skills/fsi-buyer-list.md"));
    skills.insert("fsi-cim-builder", include_str!("../skills/fsi-cim-builder.md"));
    skills.insert("fsi-datapack-builder", include_str!("../skills/fsi-datapack-builder.md"));
    skills.insert("fsi-deal-tracker", include_str!("../skills/fsi-deal-tracker.md"));
    skills.insert("fsi-merger-model", include_str!("../skills/fsi-merger-model.md"));
    skills.insert("fsi-pitch-deck", include_str!("../skills/fsi-pitch-deck.md"));
    skills.insert("fsi-process-letter", include_str!("../skills/fsi-process-letter.md"));
    skills.insert("fsi-strip-profile", include_str!("../skills/fsi-strip-profile.md"));
    skills.insert("fsi-teaser", include_str!("../skills/fsi-teaser.md"));

    // Equity Research
    skills.insert("fsi-catalyst-calendar", include_str!("../skills/fsi-catalyst-calendar.md"));
    skills.insert("fsi-earnings-analysis", include_str!("../skills/fsi-earnings-analysis.md"));
    skills.insert("fsi-earnings-preview", include_str!("../skills/fsi-earnings-preview.md"));
    skills.insert("fsi-idea-generation", include_str!("../skills/fsi-idea-generation.md"));
    skills.insert("fsi-initiating-coverage", include_str!("../skills/fsi-initiating-coverage.md"));
    skills.insert("fsi-model-update", include_str!("../skills/fsi-model-update.md"));
    skills.insert("fsi-morning-note", include_str!("../skills/fsi-morning-note.md"));
    skills.insert("fsi-sector-overview", include_str!("../skills/fsi-sector-overview.md"));
    skills.insert("fsi-thesis-tracker", include_str!("../skills/fsi-thesis-tracker.md"));

    // Private Equity
    skills.insert("fsi-dd-checklist", include_str!("../skills/fsi-dd-checklist.md"));
    skills.insert("fsi-dd-meeting-prep", include_str!("../skills/fsi-dd-meeting-prep.md"));
    skills.insert("fsi-deal-screening", include_str!("../skills/fsi-deal-screening.md"));
    skills.insert("fsi-deal-sourcing", include_str!("../skills/fsi-deal-sourcing.md"));
    skills.insert("fsi-ic-memo", include_str!("../skills/fsi-ic-memo.md"));
    skills.insert("fsi-portfolio-monitoring", include_str!("../skills/fsi-portfolio-monitoring.md"));
    skills.insert("fsi-returns-analysis", include_str!("../skills/fsi-returns-analysis.md"));
    skills.insert("fsi-unit-economics", include_str!("../skills/fsi-unit-economics.md"));
    skills.insert("fsi-value-creation-plan", include_str!("../skills/fsi-value-creation-plan.md"));

    // Wealth Management
    skills.insert("fsi-client-report", include_str!("../skills/fsi-client-report.md"));
    skills.insert("fsi-client-review", include_str!("../skills/fsi-client-review.md"));
    skills.insert("fsi-financial-plan", include_str!("../skills/fsi-financial-plan.md"));
    skills.insert("fsi-investment-proposal", include_str!("../skills/fsi-investment-proposal.md"));
    skills.insert("fsi-portfolio-rebalance", include_str!("../skills/fsi-portfolio-rebalance.md"));
    skills.insert("fsi-tax-loss-harvesting", include_str!("../skills/fsi-tax-loss-harvesting.md"));

    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_skills_loaded() {
        let skills = get_financial_services_skills();
        assert_eq!(skills.len(), FSI_SKILL_NAMES.len());
        for name in FSI_SKILL_NAMES {
            assert!(skills.contains_key(name), "Missing skill: {}", name);
        }
    }

    #[test]
    fn test_skills_have_valid_frontmatter() {
        let skills = get_financial_services_skills();
        for (name, content) in &skills {
            assert!(content.starts_with("---"), "Skill '{}' missing frontmatter", name);
            let parts: Vec<&str> = content.splitn(3, "---").collect();
            assert!(parts.len() >= 3, "Skill '{}' has malformed frontmatter", name);
            let fm = parts[1];
            assert!(fm.contains("name:"), "Skill '{}' missing name in frontmatter", name);
            assert!(fm.contains("description:"), "Skill '{}' missing description in frontmatter", name);
        }
    }
}
