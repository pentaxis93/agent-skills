//! Graph view - visual dependency graph

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::config::Config;
use crate::graph::{EdgeKind, SkillGraph};
use crate::skill::{self, Skill};

/// State for the graph view
pub struct GraphViewState {
    /// List selection state
    pub list_state: ListState,
    /// Built graph
    graph: Option<SkillGraph>,
    /// Current search filter
    pub filter: String,
}

impl GraphViewState {
    /// Create a new graph view state
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        GraphViewState {
            list_state: state,
            graph: None,
            filter: String::new(),
        }
    }

    /// Refresh the graph from current skills
    pub fn refresh(&mut self, _config: &Config, skills: &[Skill]) {
        // Build cross-reference map
        let mut crossrefs = std::collections::HashMap::new();
        let skill_names: std::collections::HashSet<String> =
            skills.iter().map(|s| s.name.clone()).collect();
        for skill in skills {
            if let Ok(content) = std::fs::read_to_string(&skill.skill_file) {
                let refs = skill::extract_references_with_filter(
                    &content,
                    &skill.name,
                    Some(&skill_names),
                );
                if !refs.is_empty() {
                    crossrefs.insert(skill.name.clone(), refs);
                }
            }
        }

        // Build graph
        self.graph = Some(SkillGraph::from_skills(&crossrefs, skills));

        // Reset selection
        if !skills.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Get the currently selected skill name
    pub fn selected_skill_name(&self) -> Option<String> {
        let graph = self.graph.as_ref()?;
        let names = graph.node_names();
        let idx = self.list_state.selected()?;
        names.get(idx).cloned()
    }

    /// Move selection down
    pub fn next(&mut self) {
        if let Some(graph) = &self.graph {
            let node_count = graph.node_count();
            if node_count == 0 {
                return;
            }
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= node_count - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    /// Move selection up
    pub fn previous(&mut self) {
        if let Some(graph) = &self.graph {
            let node_count = graph.node_count();
            if node_count == 0 {
                return;
            }
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        node_count - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }
}

/// Render the graph view
pub fn render(f: &mut Frame, area: Rect, state: &mut GraphViewState) {
    if state.graph.is_none() {
        render_empty_state(f, area);
        return;
    }

    let graph = state.graph.as_ref().unwrap();

    // Build list items showing skills with their dependencies
    let names = graph.node_names();
    let items: Vec<ListItem> = names
        .iter()
        .map(|name| {
            let mut spans = vec![];

            // Skill name with role indicator
            let color = if graph.roots.contains(name) {
                Color::LightBlue
            } else if graph.leaves.contains(name) {
                Color::LightGreen
            } else if graph.bridges.contains(name) {
                Color::Yellow
            } else {
                Color::White
            };

            spans.push(Span::styled(
                name.clone(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));

            // Show outgoing edges
            if let Some(edges) = graph.edges_from(name) {
                if !edges.is_empty() {
                    spans.push(Span::raw(" → "));
                    let targets: Vec<String> = edges
                        .iter()
                        .map(|(target, kind)| match kind {
                            EdgeKind::CrossRef => target.clone(),
                            EdgeKind::Pipeline => format!("{}(p)", target),
                        })
                        .collect();
                    spans.push(Span::styled(
                        targets.join(", "),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = format!(
        " Dependency Graph ({} nodes, {} edges) ",
        graph.node_count(),
        graph.edge_count()
    );

    let legend = format!(
        "Legend: roots={}, leaves={}, bridges={}, clusters={}",
        graph.roots.len(),
        graph.leaves.len(),
        graph.bridges.len(),
        graph.clusters.len()
    );

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .title_bottom(legend)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut state.list_state);
}

/// Render empty state when no graph is available
fn render_empty_state(f: &mut Frame, area: Rect) {
    let paragraph = ratatui::widgets::Paragraph::new(
        "No graph data available.\n\nPress 'r' to build the graph.",
    )
    .block(
        Block::default()
            .title(" Dependency Graph ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_initialize_with_no_graph() {
        // Given / When
        let state = GraphViewState::new();

        // Then
        assert!(state.graph.is_none());
        assert_eq!(state.list_state.selected(), Some(0));
    }

    #[test]
    fn should_move_selection_down() {
        // Given
        let mut state = GraphViewState::new();
        state.graph = Some(test_graph());
        state.list_state.select(Some(0));

        // When
        state.next();

        // Then
        assert_eq!(state.list_state.selected(), Some(1));
    }

    #[test]
    fn should_wrap_selection_at_end() {
        // Given
        let mut state = GraphViewState::new();
        state.graph = Some(test_graph());
        let node_count = state.graph.as_ref().unwrap().node_count();
        state.list_state.select(Some(node_count - 1));

        // When
        state.next();

        // Then (should wrap to first)
        assert_eq!(state.list_state.selected(), Some(0));
    }

    #[test]
    fn should_move_selection_up() {
        // Given
        let mut state = GraphViewState::new();
        state.graph = Some(test_graph());
        state.list_state.select(Some(1));

        // When
        state.previous();

        // Then
        assert_eq!(state.list_state.selected(), Some(0));
    }

    #[test]
    fn should_wrap_selection_at_start() {
        // Given
        let mut state = GraphViewState::new();
        state.graph = Some(test_graph());
        state.list_state.select(Some(0));
        let node_count = state.graph.as_ref().unwrap().node_count();

        // When
        state.previous();

        // Then (should wrap to last)
        assert_eq!(state.list_state.selected(), Some(node_count - 1));
    }

    fn test_graph() -> SkillGraph {
        use crate::skill::CrossRef;
        use std::collections::HashMap;

        let mut crossrefs = HashMap::new();
        crossrefs.insert(
            "skill-a".to_string(),
            vec![CrossRef {
                target: "skill-b".to_string(),
                line: 1,
                method: crate::skill::DetectionMethod::XmlCrossref,
            }],
        );

        SkillGraph::from_crossrefs(&crossrefs)
    }
}
