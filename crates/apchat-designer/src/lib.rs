//! Designer skills for APChat.
//!
//! Adapted from <https://github.com/Owl-Listener/designer-skills> (MIT licensed).
//! Provides 63 embedded skills across 8 plugins covering all major design disciplines:
//!
//! - **design-research** (10): user personas, empathy maps, journey maps, interviews, usability testing
//! - **design-systems** (8): tokens, components, accessibility, theming, documentation
//! - **ux-strategy** (8): competitive analysis, design principles, experience mapping, metrics
//! - **ui-design** (9): color systems, typography, layout grids, responsive design, data visualization
//! - **interaction-design** (7): micro-animations, state machines, gestures, error handling, feedback
//! - **prototyping-testing** (8): prototyping strategies, usability testing, heuristic evaluation, A/B experiments
//! - **design-ops** (7): critique frameworks, handoff specs, sprint planning, team workflows
//! - **designer-toolkit** (6): design rationale, presentations, case studies, UX writing, system adoption
//!
//! ## Usage
//!
//! To load all designer skills:
//! ```ignore
//! let skills = get_designer_skills(&[]);  // empty = all plugins
//! ```
//!
//! To load specific plugins:
//! ```ignore
//! let skills = get_designer_skills(&["design-research", "ui-design"]);
//! ```

use std::collections::HashMap;

/// All valid plugin names for the designer skills collection.
pub const DESIGNER_PLUGIN_NAMES: &[&str] = &[
    "design-research",
    "design-systems",
    "ux-strategy",
    "ui-design",
    "interaction-design",
    "prototyping-testing",
    "design-ops",
    "designer-toolkit",
];

/// All designer skill names grouped by plugin, for bulk operations.
pub const DESIGN_RESEARCH_SKILLS: &[&str] = &[
    "user-persona",
    "empathy-map",
    "journey-map",
    "interview-script",
    "summarize-interview",
    "usability-test-plan",
    "card-sort-analysis",
    "diary-study-plan",
    "affinity-diagram",
    "jobs-to-be-done",
];

pub const DESIGN_SYSTEMS_SKILLS: &[&str] = &[
    "accessibility-audit",
    "component-spec",
    "design-token",
    "documentation-template",
    "icon-system",
    "naming-convention",
    "pattern-library",
    "theming-system",
];

pub const UX_STRATEGY_SKILLS: &[&str] = &[
    "competitive-analysis",
    "design-brief",
    "design-principles",
    "experience-map",
    "metrics-definition",
    "north-star-vision",
    "opportunity-framework",
    "stakeholder-alignment",
];

pub const UI_DESIGN_SKILLS: &[&str] = &[
    "color-system",
    "dark-mode-design",
    "data-visualization",
    "illustration-style",
    "layout-grid",
    "responsive-design",
    "spacing-system",
    "typography-scale",
    "visual-hierarchy",
];

pub const INTERACTION_DESIGN_SKILLS: &[&str] = &[
    "animation-principles",
    "error-handling-ux",
    "feedback-patterns",
    "gesture-patterns",
    "loading-states",
    "micro-interaction-spec",
    "state-machine",
];

pub const PROTOTYPING_TESTING_SKILLS: &[&str] = &[
    "a-b-test-design",
    "accessibility-test-plan",
    "click-test-plan",
    "heuristic-evaluation",
    "prototype-strategy",
    "test-scenario",
    "user-flow-diagram",
    "wireframe-spec",
];

pub const DESIGN_OPS_SKILLS: &[&str] = &[
    "design-critique",
    "design-qa-checklist",
    "design-review-process",
    "design-sprint-plan",
    "handoff-spec",
    "team-workflow",
    "version-control-strategy",
];

pub const DESIGNER_TOOLKIT_SKILLS: &[&str] = &[
    "case-study",
    "design-rationale",
    "design-system-adoption",
    "design-token-audit",
    "presentation-deck",
    "ux-writing",
];

/// Returns designer skills as a map of name → SKILL.md content.
///
/// `plugins` controls which plugin groups to include:
/// - Empty slice (`&[]`): loads all 63 skills from all 8 plugins
/// - Non-empty slice: loads only skills from the named plugins
///
/// Valid plugin names: design-research, design-systems, ux-strategy, ui-design,
/// interaction-design, prototyping-testing, design-ops, designer-toolkit
///
/// Unknown plugin names are silently ignored. Use [`DESIGNER_PLUGIN_NAMES`] to
/// validate inputs before calling this function if strict error handling is needed.
pub fn get_designer_skills(plugins: &[&str]) -> HashMap<&'static str, &'static str> {
    let load_all = plugins.is_empty();

    let mut skills = HashMap::new();

    let should_load = |plugin: &str| load_all || plugins.contains(&plugin);

    // design-research
    if should_load("design-research") {
        skills.insert("user-persona", include_str!("../skills/user-persona.md"));
        skills.insert("empathy-map", include_str!("../skills/empathy-map.md"));
        skills.insert("journey-map", include_str!("../skills/journey-map.md"));
        skills.insert("interview-script", include_str!("../skills/interview-script.md"));
        skills.insert("summarize-interview", include_str!("../skills/summarize-interview.md"));
        skills.insert("usability-test-plan", include_str!("../skills/usability-test-plan.md"));
        skills.insert("card-sort-analysis", include_str!("../skills/card-sort-analysis.md"));
        skills.insert("diary-study-plan", include_str!("../skills/diary-study-plan.md"));
        skills.insert("affinity-diagram", include_str!("../skills/affinity-diagram.md"));
        skills.insert("jobs-to-be-done", include_str!("../skills/jobs-to-be-done.md"));
    }

    // design-systems
    if should_load("design-systems") {
        skills.insert("accessibility-audit", include_str!("../skills/accessibility-audit.md"));
        skills.insert("component-spec", include_str!("../skills/component-spec.md"));
        skills.insert("design-token", include_str!("../skills/design-token.md"));
        skills.insert("documentation-template", include_str!("../skills/documentation-template.md"));
        skills.insert("icon-system", include_str!("../skills/icon-system.md"));
        skills.insert("naming-convention", include_str!("../skills/naming-convention.md"));
        skills.insert("pattern-library", include_str!("../skills/pattern-library.md"));
        skills.insert("theming-system", include_str!("../skills/theming-system.md"));
    }

    // ux-strategy
    if should_load("ux-strategy") {
        skills.insert("competitive-analysis", include_str!("../skills/competitive-analysis.md"));
        skills.insert("design-brief", include_str!("../skills/design-brief.md"));
        skills.insert("design-principles", include_str!("../skills/design-principles.md"));
        skills.insert("experience-map", include_str!("../skills/experience-map.md"));
        skills.insert("metrics-definition", include_str!("../skills/metrics-definition.md"));
        skills.insert("north-star-vision", include_str!("../skills/north-star-vision.md"));
        skills.insert("opportunity-framework", include_str!("../skills/opportunity-framework.md"));
        skills.insert("stakeholder-alignment", include_str!("../skills/stakeholder-alignment.md"));
    }

    // ui-design
    if should_load("ui-design") {
        skills.insert("color-system", include_str!("../skills/color-system.md"));
        skills.insert("dark-mode-design", include_str!("../skills/dark-mode-design.md"));
        skills.insert("data-visualization", include_str!("../skills/data-visualization.md"));
        skills.insert("illustration-style", include_str!("../skills/illustration-style.md"));
        skills.insert("layout-grid", include_str!("../skills/layout-grid.md"));
        skills.insert("responsive-design", include_str!("../skills/responsive-design.md"));
        skills.insert("spacing-system", include_str!("../skills/spacing-system.md"));
        skills.insert("typography-scale", include_str!("../skills/typography-scale.md"));
        skills.insert("visual-hierarchy", include_str!("../skills/visual-hierarchy.md"));
    }

    // interaction-design
    if should_load("interaction-design") {
        skills.insert("animation-principles", include_str!("../skills/animation-principles.md"));
        skills.insert("error-handling-ux", include_str!("../skills/error-handling-ux.md"));
        skills.insert("feedback-patterns", include_str!("../skills/feedback-patterns.md"));
        skills.insert("gesture-patterns", include_str!("../skills/gesture-patterns.md"));
        skills.insert("loading-states", include_str!("../skills/loading-states.md"));
        skills.insert("micro-interaction-spec", include_str!("../skills/micro-interaction-spec.md"));
        skills.insert("state-machine", include_str!("../skills/state-machine.md"));
    }

    // prototyping-testing
    if should_load("prototyping-testing") {
        skills.insert("a-b-test-design", include_str!("../skills/a-b-test-design.md"));
        skills.insert("accessibility-test-plan", include_str!("../skills/accessibility-test-plan.md"));
        skills.insert("click-test-plan", include_str!("../skills/click-test-plan.md"));
        skills.insert("heuristic-evaluation", include_str!("../skills/heuristic-evaluation.md"));
        skills.insert("prototype-strategy", include_str!("../skills/prototype-strategy.md"));
        skills.insert("test-scenario", include_str!("../skills/test-scenario.md"));
        skills.insert("user-flow-diagram", include_str!("../skills/user-flow-diagram.md"));
        skills.insert("wireframe-spec", include_str!("../skills/wireframe-spec.md"));
    }

    // design-ops
    if should_load("design-ops") {
        skills.insert("design-critique", include_str!("../skills/design-critique.md"));
        skills.insert("design-qa-checklist", include_str!("../skills/design-qa-checklist.md"));
        skills.insert("design-review-process", include_str!("../skills/design-review-process.md"));
        skills.insert("design-sprint-plan", include_str!("../skills/design-sprint-plan.md"));
        skills.insert("handoff-spec", include_str!("../skills/handoff-spec.md"));
        skills.insert("team-workflow", include_str!("../skills/team-workflow.md"));
        skills.insert("version-control-strategy", include_str!("../skills/version-control-strategy.md"));
    }

    // designer-toolkit
    if should_load("designer-toolkit") {
        skills.insert("case-study", include_str!("../skills/case-study.md"));
        skills.insert("design-rationale", include_str!("../skills/design-rationale.md"));
        skills.insert("design-system-adoption", include_str!("../skills/design-system-adoption.md"));
        skills.insert("design-token-audit", include_str!("../skills/design-token-audit.md"));
        skills.insert("presentation-deck", include_str!("../skills/presentation-deck.md"));
        skills.insert("ux-writing", include_str!("../skills/ux-writing.md"));
    }

    skills
}
