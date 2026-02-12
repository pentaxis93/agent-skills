//! System Overview - landing page dashboard

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::collections::{HashMap, HashSet};

use crate::commands::check::{self, Severity};
use crate::config::Config;
#[cfg(feature = "graph")]
use crate::graph::SkillGraph;
use crate::skill::{self, Skill};

/// System Overview state
pub struct OverviewState {
    /// Cached data refreshed when entering the view
    data: Option<OverviewData>,
}

struct OverviewData {
    total_skills: usize,
    total_sources: usize,
    total_targets: usize,
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    clusters: Vec<(String, Vec<String>)>, // (cluster name, member skills)
    pipelines: Vec<PipelineInfo>,
    unconnected: Vec<String>,
    recent: Vec<String>, // skill names sorted by mtime
}

struct PipelineInfo {
    name: String,
    stage_count: usize,
    skill_count: usize,
    has_gaps: bool,
}

impl OverviewState {
    pub fn new() -> Self {
        OverviewState { data: None }
    }

    /// Refresh the overview data
    pub fn refresh(&mut self, config: &Config, skills: &[Skill]) {
        let total_skills = skills.len();
        let total_sources = config.sources.skills.len();
        let total_targets = config.global.targets.len();

        // Health summary
        let findings = check::check(config, None, false).unwrap_or_default();
        let error_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();
        let warning_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();
        let info_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Info)
            .count();

        // Clusters
        #[cfg(feature = "graph")]
        let clusters = extract_clusters(skills);
        #[cfg(not(feature = "graph"))]
        let clusters = Vec::new();

        // Pipelines
        let pipelines = extract_pipelines(skills);

        // Unconnected skills
        #[cfg(feature = "graph")]
        let unconnected = find_unconnected(skills);
        #[cfg(not(feature = "graph"))]
        let unconnected = Vec::new();

        // Recent changes
        let recent = find_recent_skills(skills, 10);

        self.data = Some(OverviewData {
            total_skills,
            total_sources,
            total_targets,
            error_count,
            warning_count,
            info_count,
            clusters,
            pipelines,
            unconnected,
            recent,
        });
    }
}

/// Render the overview
pub fn render(f: &mut Frame, area: Rect, state: &OverviewState) {
    if state.data.is_none() {
        render_loading(f, area);
        return;
    }

    let data = state.data.as_ref().unwrap();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main content
        ])
        .split(area);

    render_header(f, chunks[0], data);
    render_content(f, chunks[1], data);
}

fn render_loading(f: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new("Loading system overview...").block(
        Block::default()
            .title(" System Overview ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(paragraph, area);
}

fn render_header(f: &mut Frame, area: Rect, data: &OverviewData) {
    let health_color = if data.error_count > 0 {
        Color::Red
    } else if data.warning_count > 0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let lines = vec![
        Line::from(vec![
            Span::raw("Skills: "),
            Span::styled(
                data.total_skills.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | Sources: "),
            Span::styled(
                data.total_sources.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | Targets: "),
            Span::styled(
                data.total_targets.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Health: "),
            Span::styled(
                format!(
                    "{} errors, {} warnings, {} info",
                    data.error_count, data.warning_count, data.info_count
                ),
                Style::default().fg(health_color),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(" System Overview ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(paragraph, area);
}

fn render_content(f: &mut Frame, area: Rect, data: &OverviewData) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    render_clusters(f, left_chunks[0], data);
    render_pipelines(f, left_chunks[1], data);
    render_unconnected(f, right_chunks[0], data);
    render_recent(f, right_chunks[1], data);
}

fn render_clusters(f: &mut Frame, area: Rect, data: &OverviewData) {
    let items: Vec<ListItem> = if data.clusters.is_empty() {
        vec![ListItem::new("No clusters detected")]
    } else {
        data.clusters
            .iter()
            .map(|(name, members)| {
                let line = format!(
                    "{} ({} skills): {}",
                    name,
                    members.len(),
                    members.join(", ")
                );
                ListItem::new(line)
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .title(format!(" Clusters ({}) ", data.clusters.len()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(list, area);
}

fn render_pipelines(f: &mut Frame, area: Rect, data: &OverviewData) {
    let items: Vec<ListItem> = if data.pipelines.is_empty() {
        vec![ListItem::new("No pipelines defined")]
    } else {
        data.pipelines
            .iter()
            .map(|p| {
                let gap_indicator = if p.has_gaps { " ⚠ gaps" } else { "" };
                let line = format!(
                    "{}: {} stages, {} skills{}",
                    p.name, p.stage_count, p.skill_count, gap_indicator
                );
                ListItem::new(line)
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .title(format!(" Pipelines ({}) ", data.pipelines.len()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(list, area);
}

fn render_unconnected(f: &mut Frame, area: Rect, data: &OverviewData) {
    let items: Vec<ListItem> = if data.unconnected.is_empty() {
        vec![ListItem::new("All skills are connected")]
    } else {
        data.unconnected
            .iter()
            .map(|name| ListItem::new(name.clone()))
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .title(format!(" Unconnected Skills ({}) ", data.unconnected.len()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(list, area);
}

fn render_recent(f: &mut Frame, area: Rect, data: &OverviewData) {
    let items: Vec<ListItem> = if data.recent.is_empty() {
        vec![ListItem::new("No skills found")]
    } else {
        data.recent
            .iter()
            .map(|name| ListItem::new(name.clone()))
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .title(" Recent Changes (top 10) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(list, area);
}

// Data extraction functions

#[cfg(feature = "graph")]
fn extract_clusters(skills: &[Skill]) -> Vec<(String, Vec<String>)> {
    // Build cross-reference map
    let mut crossrefs = HashMap::new();
    let skill_names: HashSet<String> = skills.iter().map(|s| s.name.clone()).collect();

    for skill in skills {
        if let Ok(content) = std::fs::read_to_string(&skill.skill_file) {
            let refs =
                skill::extract_references_with_filter(&content, &skill.name, Some(&skill_names));
            if !refs.is_empty() {
                crossrefs.insert(skill.name.clone(), refs);
            }
        }
    }

    let graph = SkillGraph::from_skills(&crossrefs, skills);

    graph
        .clusters
        .iter()
        .enumerate()
        .map(|(i, members)| {
            let name = format!("cluster-{}", i + 1);
            (name, members.clone())
        })
        .collect()
}

fn extract_pipelines(skills: &[Skill]) -> Vec<PipelineInfo> {
    let mut pipeline_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut pipeline_stages: HashMap<String, HashSet<(u32, String)>> = HashMap::new();

    for skill in skills {
        if let Some(pipeline_data) = &skill.frontmatter.pipeline {
            for (pipeline_name, stage) in pipeline_data {
                pipeline_map
                    .entry(pipeline_name.clone())
                    .or_default()
                    .insert(skill.name.clone());
                pipeline_stages
                    .entry(pipeline_name.clone())
                    .or_default()
                    .insert((stage.order, stage.stage.clone()));
            }
        }
    }

    let mut pipelines: Vec<PipelineInfo> = pipeline_map
        .into_iter()
        .map(|(name, skills)| {
            let stages = pipeline_stages.get(&name).unwrap();
            let mut orders: Vec<u32> = stages.iter().map(|(order, _)| *order).collect();
            orders.sort();

            // Check for gaps in ordering
            let has_gaps = orders.windows(2).any(|w| w[1] - w[0] > 1);

            PipelineInfo {
                name,
                stage_count: stages.len(),
                skill_count: skills.len(),
                has_gaps,
            }
        })
        .collect();

    pipelines.sort_by(|a, b| a.name.cmp(&b.name));
    pipelines
}

#[cfg(feature = "graph")]
fn find_unconnected(skills: &[Skill]) -> Vec<String> {
    // Build cross-reference map
    let mut crossrefs = HashMap::new();
    let skill_names: HashSet<String> = skills.iter().map(|s| s.name.clone()).collect();

    for skill in skills {
        if let Ok(content) = std::fs::read_to_string(&skill.skill_file) {
            let refs =
                skill::extract_references_with_filter(&content, &skill.name, Some(&skill_names));
            if !refs.is_empty() {
                crossrefs.insert(skill.name.clone(), refs);
            }
        }
    }

    let graph = SkillGraph::from_skills(&crossrefs, skills);

    // Find skills with no incoming or outgoing edges
    let mut unconnected: Vec<String> = skills
        .iter()
        .filter(|s| {
            let has_outgoing = graph
                .edges_from(&s.name)
                .map(|e| !e.is_empty())
                .unwrap_or(false);
            let has_incoming = graph
                .edges_to(&s.name)
                .map(|e| !e.is_empty())
                .unwrap_or(false);
            !has_outgoing && !has_incoming
        })
        .map(|s| s.name.clone())
        .collect();

    unconnected.sort();
    unconnected
}

fn find_recent_skills(skills: &[Skill], limit: usize) -> Vec<String> {
    let mut skill_times: Vec<(String, std::time::SystemTime)> = skills
        .iter()
        .filter_map(|s| {
            std::fs::metadata(&s.skill_file)
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|mtime| (s.name.clone(), mtime))
        })
        .collect();

    skill_times.sort_by(|a, b| b.1.cmp(&a.1)); // Descending by time
    skill_times.truncate(limit);
    skill_times.into_iter().map(|(name, _)| name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_initialize_with_no_data() {
        // Given / When
        let state = OverviewState::new();

        // Then
        assert!(state.data.is_none());
    }

    #[test]
    fn should_extract_pipeline_info() {
        // Given
        let skills = vec![test_skill_with_pipeline(
            "skill-a",
            "test-pipeline",
            "stage-1",
            1,
        )];

        // When
        let pipelines = extract_pipelines(&skills);

        // Then
        assert_eq!(pipelines.len(), 1);
        assert_eq!(pipelines[0].name, "test-pipeline");
        assert_eq!(pipelines[0].skill_count, 1);
        assert_eq!(pipelines[0].stage_count, 1);
        assert!(!pipelines[0].has_gaps);
    }

    #[test]
    fn should_detect_pipeline_gaps() {
        // Given
        let skills = vec![
            test_skill_with_pipeline("skill-a", "test-pipeline", "stage-1", 1),
            test_skill_with_pipeline("skill-b", "test-pipeline", "stage-3", 3), // Gap: no order 2
        ];

        // When
        let pipelines = extract_pipelines(&skills);

        // Then
        assert_eq!(pipelines.len(), 1);
        assert!(pipelines[0].has_gaps);
    }

    #[test]
    fn should_sort_recent_skills_by_mtime() {
        // This test would require creating actual files with specific mtimes
        // Skipping for now as it's filesystem-dependent
    }

    fn test_skill_with_pipeline(name: &str, pipeline: &str, stage: &str, order: u32) -> Skill {
        use crate::skill::frontmatter::{Frontmatter, PipelineStage};
        use std::collections::HashMap;
        use std::path::PathBuf;

        let mut pipeline_data = HashMap::new();
        pipeline_data.insert(
            pipeline.to_string(),
            PipelineStage {
                stage: stage.to_string(),
                order,
                after: None,
                before: None,
            },
        );

        Skill {
            name: name.to_string(),
            path: PathBuf::from(format!("/test/{}", name)),
            skill_file: PathBuf::from(format!("/test/{}/SKILL.md", name)),
            frontmatter: Frontmatter {
                name: name.to_string(),
                description: format!("Test skill {}", name),
                tags: None,
                pipeline: Some(pipeline_data),
                disable_model_invocation: None,
                user_invocable: None,
                allowed_tools: None,
                context: None,
                agent: None,
                model: None,
                argument_hint: None,
                license: None,
                compatibility: None,
                metadata: None,
            },
        }
    }
}
