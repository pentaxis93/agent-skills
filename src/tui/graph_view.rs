//! Graph explorer - focused node navigation with breadcrumb trail

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::config::Config;
use crate::graph::{EdgeKind, SkillGraph};
use crate::skill::{self, Skill};

/// Navigation mode for the graph explorer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationMode {
    /// Browse mode - scrollable list of all nodes
    Browse,
    /// Focus mode - examine a single node and its edges
    Focus,
}

/// State for the graph explorer view
pub struct GraphViewState {
    /// Current navigation mode
    pub mode: NavigationMode,
    /// List selection state (for browse mode)
    pub list_state: ListState,
    /// Built graph
    graph: Option<SkillGraph>,
    /// Navigation trail (breadcrumb history)
    pub trail: Vec<String>,
    /// Edge selection state (for focus mode)
    pub edge_list_state: ListState,
}

impl GraphViewState {
    /// Create a new graph view state
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let mut edge_list_state = ListState::default();
        edge_list_state.select(Some(0));
        GraphViewState {
            mode: NavigationMode::Browse,
            list_state,
            graph: None,
            trail: Vec::new(),
            edge_list_state,
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

        // Reset state
        if !skills.is_empty() {
            self.list_state.select(Some(0));
        }
        self.trail.clear();
        self.mode = NavigationMode::Browse;
    }

    /// Get the currently focused skill name
    pub fn focused_skill(&self) -> Option<String> {
        match self.mode {
            NavigationMode::Browse => {
                let graph = self.graph.as_ref()?;
                let names = graph.node_names();
                let idx = self.list_state.selected()?;
                names.get(idx).cloned()
            }
            NavigationMode::Focus => self.trail.last().cloned(),
        }
    }

    /// Toggle between browse and focus modes
    pub fn toggle_mode(&mut self) {
        match self.mode {
            NavigationMode::Browse => {
                // Enter focus mode on current selection
                if let Some(skill) = self.focused_skill() {
                    self.trail.push(skill);
                    self.mode = NavigationMode::Focus;
                    self.edge_list_state.select(Some(0));
                }
            }
            NavigationMode::Focus => {
                // Return to browse mode
                self.mode = NavigationMode::Browse;
            }
        }
    }

    /// Navigate back in the trail
    pub fn navigate_back(&mut self) {
        if self.mode == NavigationMode::Focus && self.trail.len() > 1 {
            self.trail.pop();
            self.edge_list_state.select(Some(0));
        } else if self.mode == NavigationMode::Focus {
            self.mode = NavigationMode::Browse;
            self.trail.clear();
        }
    }

    /// Follow selected edge to target node
    pub fn follow_edge(&mut self) {
        if self.mode != NavigationMode::Focus {
            return;
        }

        let current = match self.trail.last() {
            Some(s) => s,
            None => return,
        };

        let graph = match &self.graph {
            Some(g) => g,
            None => return,
        };

        // Get all edges (outgoing + incoming)
        let mut all_edges = Vec::new();
        if let Some(outgoing) = graph.edges_from(current) {
            for (target, kind) in outgoing {
                all_edges.push((target, kind, EdgeDirection::Outgoing));
            }
        }
        if let Some(incoming) = graph.edges_to(current) {
            for (source, kind) in incoming {
                all_edges.push((source, kind, EdgeDirection::Incoming));
            }
        }

        if all_edges.is_empty() {
            return;
        }

        let idx = self.edge_list_state.selected().unwrap_or(0);
        if let Some((target, _, _)) = all_edges.get(idx) {
            self.trail.push(target.clone());
            self.edge_list_state.select(Some(0));
        }
    }

    /// Move selection down
    pub fn next(&mut self) {
        match self.mode {
            NavigationMode::Browse => {
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
            NavigationMode::Focus => {
                // Move through edges
                let current = match self.trail.last() {
                    Some(s) => s,
                    None => return,
                };
                let graph = match &self.graph {
                    Some(g) => g,
                    None => return,
                };

                let edge_count = count_edges(graph, current);
                if edge_count == 0 {
                    return;
                }

                let i = match self.edge_list_state.selected() {
                    Some(i) => {
                        if i >= edge_count - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.edge_list_state.select(Some(i));
            }
        }
    }

    /// Move selection up
    pub fn previous(&mut self) {
        match self.mode {
            NavigationMode::Browse => {
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
            NavigationMode::Focus => {
                // Move through edges
                let current = match self.trail.last() {
                    Some(s) => s,
                    None => return,
                };
                let graph = match &self.graph {
                    Some(g) => g,
                    None => return,
                };

                let edge_count = count_edges(graph, current);
                if edge_count == 0 {
                    return;
                }

                let i = match self.edge_list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            edge_count - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.edge_list_state.select(Some(i));
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum EdgeDirection {
    Outgoing,
    Incoming,
}

fn count_edges(graph: &SkillGraph, skill_name: &str) -> usize {
    let outgoing = graph.edges_from(skill_name).map(|e| e.len()).unwrap_or(0);
    let incoming = graph.edges_to(skill_name).map(|e| e.len()).unwrap_or(0);
    outgoing + incoming
}

/// Render the graph explorer view
pub fn render(f: &mut Frame, area: Rect, state: &mut GraphViewState) {
    if state.graph.is_none() {
        render_empty_state(f, area);
        return;
    }

    match state.mode {
        NavigationMode::Browse => render_browse_mode(f, area, state),
        NavigationMode::Focus => render_focus_mode(f, area, state),
    }
}

/// Render browse mode - scrollable list of all nodes
fn render_browse_mode(f: &mut Frame, area: Rect, state: &mut GraphViewState) {
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

            // Show outgoing edges count
            let out_count = graph.edges_from(name).map(|e| e.len()).unwrap_or(0);
            let in_count = graph.edges_to(name).map(|e| e.len()).unwrap_or(0);
            spans.push(Span::styled(
                format!(" (→{} ←{})", out_count, in_count),
                Style::default().fg(Color::DarkGray),
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = format!(
        " Graph Explorer - Browse ({} nodes, {} edges) ",
        graph.node_count(),
        graph.edge_count()
    );

    let legend = format!(
        "Enter: focus node | Legend: roots={}, leaves={}, bridges={}, clusters={}",
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

/// Render focus mode - detailed node view with edge navigation
fn render_focus_mode(f: &mut Frame, area: Rect, state: &mut GraphViewState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Breadcrumb trail
            Constraint::Length(8), // Current node info
            Constraint::Min(0),    // Edges list
        ])
        .split(area);

    render_breadcrumb_trail(f, chunks[0], state);
    render_node_info(f, chunks[1], state);
    render_edge_list(f, chunks[2], state);
}

/// Render the breadcrumb trail showing navigation history
fn render_breadcrumb_trail(f: &mut Frame, area: Rect, state: &GraphViewState) {
    let trail_text = if state.trail.is_empty() {
        "No navigation history".to_string()
    } else {
        state.trail.join(" → ")
    };

    let paragraph = Paragraph::new(trail_text)
        .block(
            Block::default()
                .title(" Navigation Trail (Backspace: back, Esc: return to browse) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Render current node information
fn render_node_info(f: &mut Frame, area: Rect, state: &GraphViewState) {
    let graph = state.graph.as_ref().unwrap();
    let current = match state.trail.last() {
        Some(s) => s,
        None => {
            let paragraph = Paragraph::new("No node selected")
                .block(Block::default().title(" Node Info ").borders(Borders::ALL));
            f.render_widget(paragraph, area);
            return;
        }
    };

    let mut lines = vec![];
    lines.push(format!("Skill: {}", current));

    // Role badges
    let mut roles = vec![];
    if graph.roots.contains(current) {
        roles.push("Root");
    }
    if graph.leaves.contains(current) {
        roles.push("Leaf");
    }
    if graph.bridges.contains(current) {
        roles.push("Bridge");
    }
    if !roles.is_empty() {
        lines.push(format!("Roles: {}", roles.join(", ")));
    }

    // Edge counts
    let out_count = graph.edges_from(current).map(|e| e.len()).unwrap_or(0);
    let in_count = graph.edges_to(current).map(|e| e.len()).unwrap_or(0);
    lines.push(format!("Outgoing: {} | Incoming: {}", out_count, in_count));

    // Cluster membership
    for cluster in &graph.clusters {
        if cluster.contains(current) {
            lines.push(format!("Cluster: {} members", cluster.len()));
            break;
        }
    }

    let paragraph = Paragraph::new(lines.join("\n"))
        .block(
            Block::default()
                .title(" Node Info ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Render the edge list for the current node
fn render_edge_list(f: &mut Frame, area: Rect, state: &mut GraphViewState) {
    let graph = state.graph.as_ref().unwrap();
    let current = match state.trail.last() {
        Some(s) => s,
        None => return,
    };

    // Collect all edges (outgoing + incoming)
    let mut all_edges = Vec::new();

    if let Some(outgoing) = graph.edges_from(current) {
        for (target, kind) in outgoing {
            all_edges.push((target, kind, EdgeDirection::Outgoing));
        }
    }

    if let Some(incoming) = graph.edges_to(current) {
        for (source, kind) in incoming {
            all_edges.push((source, kind, EdgeDirection::Incoming));
        }
    }

    if all_edges.is_empty() {
        let paragraph = Paragraph::new("No edges from this node.\n\nPress Esc to return.").block(
            Block::default()
                .title(" Edges ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        f.render_widget(paragraph, area);
        return;
    }

    // Build list items
    let items: Vec<ListItem> = all_edges
        .iter()
        .map(|(target, kind, direction)| {
            let arrow = match direction {
                EdgeDirection::Outgoing => "→",
                EdgeDirection::Incoming => "←",
            };
            let kind_label = match kind {
                EdgeKind::CrossRef => "ref",
                EdgeKind::Pipeline => "pipeline",
            };
            let color = match direction {
                EdgeDirection::Outgoing => Color::Cyan,
                EdgeDirection::Incoming => Color::Magenta,
            };

            let line = Line::from(vec![
                Span::styled(arrow, Style::default().fg(color)),
                Span::raw(" "),
                Span::raw(target.clone()),
                Span::raw(" "),
                Span::styled(
                    format!("({})", kind_label),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let title = format!(" Edges ({}) - Enter: follow edge ", all_edges.len());

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut state.edge_list_state);
}

/// Render empty state when no graph is available
fn render_empty_state(f: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new("No graph data available.\n\nPress 'r' to build the graph.")
        .block(
            Block::default()
                .title(" Graph Explorer ")
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
        assert_eq!(state.mode, NavigationMode::Browse);
        assert!(state.trail.is_empty());
    }

    #[test]
    fn should_move_selection_down_in_browse_mode() {
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

    #[test]
    fn should_toggle_to_focus_mode() {
        // Given
        let mut state = GraphViewState::new();
        state.graph = Some(test_graph());
        state.list_state.select(Some(0));

        // When
        state.toggle_mode();

        // Then
        assert_eq!(state.mode, NavigationMode::Focus);
        assert_eq!(state.trail.len(), 1);
    }

    #[test]
    fn should_navigate_back_in_trail() {
        // Given
        let mut state = GraphViewState::new();
        state.graph = Some(test_graph());
        state.mode = NavigationMode::Focus;
        state.trail = vec!["skill-a".to_string(), "skill-b".to_string()];

        // When
        state.navigate_back();

        // Then
        assert_eq!(state.trail.len(), 1);
        assert_eq!(state.trail[0], "skill-a");
    }

    #[test]
    fn should_return_to_browse_when_trail_empty() {
        // Given
        let mut state = GraphViewState::new();
        state.graph = Some(test_graph());
        state.mode = NavigationMode::Focus;
        state.trail = vec!["skill-a".to_string()];

        // When
        state.navigate_back();

        // Then
        assert_eq!(state.mode, NavigationMode::Browse);
        assert!(state.trail.is_empty());
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
