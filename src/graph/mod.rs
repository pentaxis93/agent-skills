//! Dependency graph construction and analysis (requires `graph` feature)

use petgraph::algo::tarjan_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};

use crate::skill::{CrossRef, Skill};

/// Edge type in the skill graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// Detected from content cross-references
    CrossRef,
    /// Declared in pipeline after/before fields
    Pipeline,
}

/// A skill dependency graph with analysis results
#[derive(Debug)]
pub struct SkillGraph {
    /// The underlying directed graph
    graph: DiGraph<String, EdgeKind>,

    /// Map from skill name to node index
    name_to_node: HashMap<String, NodeIndex>,

    /// Detected clusters (strongly connected components)
    pub clusters: Vec<Vec<String>>,

    /// Root skills (no incoming edges)
    pub roots: Vec<String>,

    /// Leaf skills (no outgoing edges)
    pub leaves: Vec<String>,

    /// Bridge skills (articulation points)
    pub bridges: Vec<String>,
}

impl SkillGraph {
    /// Build a skill graph from cross-reference data and skill metadata
    pub fn from_skills(crossrefs: &HashMap<String, Vec<CrossRef>>, skills: &[Skill]) -> Self {
        let mut graph = DiGraph::new();
        let mut name_to_node = HashMap::new();
        let mut edge_set: HashSet<(String, String)> = HashSet::new();

        // Collect all unique skill names from crossrefs
        let mut all_skills: HashSet<String> = HashSet::new();
        for (source, refs) in crossrefs {
            all_skills.insert(source.clone());
            for r in refs {
                all_skills.insert(r.target.clone());
            }
        }

        // Also add all discovered skills (some may have no crossrefs)
        for skill in skills {
            all_skills.insert(skill.name.clone());
        }

        // Add all skills as nodes
        let mut sorted_skills: Vec<_> = all_skills.iter().cloned().collect();
        sorted_skills.sort();
        for skill in &sorted_skills {
            let node = graph.add_node(skill.clone());
            name_to_node.insert(skill.clone(), node);
        }

        // Add deduplicated edges from cross-references
        for (source, refs) in crossrefs {
            let source_node = name_to_node[source];
            for r in refs {
                let edge_key = (source.clone(), r.target.clone());
                if !edge_set.contains(&edge_key) {
                    if let Some(&target_node) = name_to_node.get(&r.target) {
                        graph.add_edge(source_node, target_node, EdgeKind::CrossRef);
                        edge_set.insert(edge_key);
                    }
                }
            }
        }

        // Add edges from pipeline after/before declarations
        for skill in skills {
            if let Some(pipeline) = &skill.frontmatter.pipeline {
                for stage in pipeline.values() {
                    // "after" means this skill depends on those skills
                    if let Some(after) = &stage.after {
                        for dep in after {
                            let edge_key = (skill.name.clone(), dep.clone());
                            if !edge_set.contains(&edge_key) {
                                if let (Some(&source_node), Some(&target_node)) =
                                    (name_to_node.get(&skill.name), name_to_node.get(dep))
                                {
                                    graph.add_edge(source_node, target_node, EdgeKind::Pipeline);
                                    edge_set.insert(edge_key);
                                }
                            }
                        }
                    }
                    // "before" means those skills depend on this skill (reverse direction)
                    if let Some(before) = &stage.before {
                        for dep in before {
                            let edge_key = (dep.clone(), skill.name.clone());
                            if !edge_set.contains(&edge_key) {
                                if let (Some(&source_node), Some(&target_node)) =
                                    (name_to_node.get(dep), name_to_node.get(&skill.name))
                                {
                                    graph.add_edge(source_node, target_node, EdgeKind::Pipeline);
                                    edge_set.insert(edge_key);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Analyze the graph
        let clusters = detect_clusters(&graph, &name_to_node);
        let roots = find_roots(&graph, &name_to_node);
        let leaves = find_leaves(&graph, &name_to_node);
        let bridges = find_bridges(&graph, &name_to_node);

        SkillGraph {
            graph,
            name_to_node,
            clusters,
            roots,
            leaves,
            bridges,
        }
    }

    /// Build a skill graph from cross-reference data only (backward compat)
    pub fn from_crossrefs(crossrefs: &HashMap<String, Vec<CrossRef>>) -> Self {
        Self::from_skills(crossrefs, &[])
    }

    /// Filter to only skills in a specific pipeline
    pub fn filter_pipeline(&self, skills: &[Skill], pipeline_name: &str) -> Self {
        let pipeline_skills: HashSet<String> = skills
            .iter()
            .filter(|s| {
                s.frontmatter
                    .pipeline
                    .as_ref()
                    .map(|p| p.contains_key(pipeline_name))
                    .unwrap_or(false)
            })
            .map(|s| s.name.clone())
            .collect();

        self.filter_to_skills(&pipeline_skills, skills)
    }

    /// Filter to only skills with a specific tag
    pub fn filter_tag(&self, skills: &[Skill], tag: &str) -> Self {
        let tagged_skills: HashSet<String> = skills
            .iter()
            .filter(|s| {
                s.frontmatter
                    .tags
                    .as_ref()
                    .map(|t| t.contains(&tag.to_string()))
                    .unwrap_or(false)
            })
            .map(|s| s.name.clone())
            .collect();

        self.filter_to_skills(&tagged_skills, skills)
    }

    /// Create a subgraph containing only the specified skills
    fn filter_to_skills(&self, keep: &HashSet<String>, skills: &[Skill]) -> Self {
        let mut crossrefs: HashMap<String, Vec<CrossRef>> = HashMap::new();

        // Rebuild crossrefs for kept nodes only
        for (name, &idx) in &self.name_to_node {
            if !keep.contains(name) {
                continue;
            }
            let edges: Vec<CrossRef> = self
                .graph
                .edges(idx)
                .filter(|e| keep.contains(&self.graph[e.target()]))
                .map(|e| CrossRef {
                    target: self.graph[e.target()].clone(),
                    line: 0,
                    method: crate::skill::DetectionMethod::XmlCrossref,
                })
                .collect();
            if !edges.is_empty() {
                crossrefs.insert(name.clone(), edges);
            }
        }

        // Only pass skills that are in the keep set
        let filtered_skills: Vec<Skill> = skills
            .iter()
            .filter(|s| keep.contains(&s.name))
            .cloned()
            .collect();

        Self::from_skills(&crossrefs, &filtered_skills)
    }

    /// Export graph as Graphviz DOT format
    pub fn to_dot(&self) -> String {
        let mut output = String::from("digraph SkillGraph {\n");
        output.push_str("  rankdir=LR;\n");
        output.push_str("  node [shape=box, style=rounded];\n\n");

        // Add nodes
        let mut sorted: Vec<_> = self.name_to_node.iter().collect();
        sorted.sort_by_key(|(name, _)| (*name).clone());
        for (name, _) in &sorted {
            let color = if self.roots.contains(*name) {
                "lightblue"
            } else if self.leaves.contains(*name) {
                "lightgreen"
            } else if self.bridges.contains(*name) {
                "orange"
            } else {
                "white"
            };
            output.push_str(&format!(
                "  \"{}\" [fillcolor={}, style=\"rounded,filled\"];\n",
                name, color
            ));
        }

        output.push('\n');

        // Add edges with style based on kind
        for edge in self.graph.edge_references() {
            let source = &self.graph[edge.source()];
            let target = &self.graph[edge.target()];
            let style = match edge.weight() {
                EdgeKind::CrossRef => "",
                EdgeKind::Pipeline => " [style=dashed, color=blue]",
            };
            output.push_str(&format!("  \"{}\" -> \"{}\"{};\n", source, target, style));
        }

        output.push_str("}\n");
        output
    }

    /// Export graph as human-readable adjacency list
    pub fn to_text(&self) -> String {
        let mut output = String::new();

        output.push_str("# Skill Dependency Graph\n\n");

        // Show analysis summary
        output.push_str(&format!("Skills: {}\n", self.name_to_node.len()));
        output.push_str(&format!("Clusters: {}\n", self.clusters.len()));
        output.push_str(&format!("Roots: {}\n", self.roots.len()));
        output.push_str(&format!("Leaves: {}\n", self.leaves.len()));
        output.push_str(&format!("Bridges: {}\n\n", self.bridges.len()));

        // Show adjacency list
        output.push_str("## Dependencies\n\n");
        let mut sorted_skills: Vec<_> = self.name_to_node.keys().collect();
        sorted_skills.sort();

        for skill in sorted_skills {
            let node = self.name_to_node[skill];
            let mut targets: Vec<String> = self
                .graph
                .edges(node)
                .map(|e| self.graph[e.target()].clone())
                .collect();
            targets.sort();
            targets.dedup();

            if targets.is_empty() {
                output.push_str(&format!("{}: (none)\n", skill));
            } else {
                output.push_str(&format!("{}: {}\n", skill, targets.join(", ")));
            }
        }

        output
    }

    /// Export graph as JSON
    pub fn to_json(&self) -> String {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let mut sorted: Vec<_> = self.name_to_node.iter().collect();
        sorted.sort_by_key(|(name, _)| (*name).clone());

        for (name, &idx) in &sorted {
            nodes.push(serde_json::json!({
                "id": name,
                "is_root": self.roots.contains(*name),
                "is_leaf": self.leaves.contains(*name),
                "is_bridge": self.bridges.contains(*name),
            }));

            for edge in self.graph.edges(idx) {
                let target = &self.graph[edge.target()];
                let kind = match edge.weight() {
                    EdgeKind::CrossRef => "crossref",
                    EdgeKind::Pipeline => "pipeline",
                };
                edges.push(serde_json::json!({
                    "source": name,
                    "target": target,
                    "kind": kind,
                }));
            }
        }

        serde_json::json!({
            "nodes": nodes,
            "edges": edges,
            "clusters": self.clusters,
        })
        .to_string()
    }

    /// Export graph as Mermaid diagram
    pub fn to_mermaid(&self) -> String {
        let mut output = String::from("graph LR\n");
        let mut seen_edges: HashSet<(String, String)> = HashSet::new();

        for edge in self.graph.edge_references() {
            let source = &self.graph[edge.source()];
            let target = &self.graph[edge.target()];
            let key = (source.clone(), target.clone());
            if seen_edges.contains(&key) {
                continue;
            }
            seen_edges.insert(key);

            let arrow = match edge.weight() {
                EdgeKind::CrossRef => "-->",
                EdgeKind::Pipeline => "-.->",
            };
            output.push_str(&format!(
                "  {}[{}] {} {}[{}]\n",
                sanitize_mermaid(source),
                source,
                arrow,
                sanitize_mermaid(target),
                target
            ));
        }

        output
    }
}

fn sanitize_mermaid(s: &str) -> String {
    s.replace('-', "_")
}

fn detect_clusters(
    graph: &DiGraph<String, EdgeKind>,
    _name_to_node: &HashMap<String, NodeIndex>,
) -> Vec<Vec<String>> {
    // Use Tarjan's algorithm to find strongly connected components
    let sccs = tarjan_scc(graph);

    let mut clusters = Vec::new();
    for scc in sccs {
        let cluster: Vec<String> = scc.iter().map(|&idx| graph[idx].clone()).collect();

        // Only include clusters with more than one skill
        if cluster.len() > 1 {
            clusters.push(cluster);
        }
    }

    clusters
}

fn find_roots(
    graph: &DiGraph<String, EdgeKind>,
    name_to_node: &HashMap<String, NodeIndex>,
) -> Vec<String> {
    let mut roots = Vec::new();

    for (name, &idx) in name_to_node {
        // Root skills have no incoming edges
        if graph
            .edges_directed(idx, petgraph::Direction::Incoming)
            .count()
            == 0
        {
            roots.push(name.clone());
        }
    }

    roots.sort();
    roots
}

fn find_leaves(
    graph: &DiGraph<String, EdgeKind>,
    name_to_node: &HashMap<String, NodeIndex>,
) -> Vec<String> {
    let mut leaves = Vec::new();

    for (name, &idx) in name_to_node {
        // Leaf skills have no outgoing edges
        if graph
            .edges_directed(idx, petgraph::Direction::Outgoing)
            .count()
            == 0
        {
            leaves.push(name.clone());
        }
    }

    leaves.sort();
    leaves
}

fn find_bridges(
    graph: &DiGraph<String, EdgeKind>,
    name_to_node: &HashMap<String, NodeIndex>,
) -> Vec<String> {
    // Articulation points - nodes whose removal would increase connected components
    // For directed graphs, this is approximate - we look for nodes that are the only path
    // between different parts of the graph

    let mut bridges = Vec::new();

    // Simple heuristic: a node is a bridge if it has both incoming and outgoing edges
    // and removing it would disconnect some nodes
    for (name, &idx) in name_to_node {
        let incoming = graph
            .edges_directed(idx, petgraph::Direction::Incoming)
            .count();
        let outgoing = graph
            .edges_directed(idx, petgraph::Direction::Outgoing)
            .count();

        // Bridge candidates have both incoming and outgoing edges
        if incoming > 0 && outgoing > 0 {
            bridges.push(name.clone());
        }
    }

    bridges.sort();
    bridges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{CrossRef, DetectionMethod};

    fn test_crossref(target: &str) -> CrossRef {
        CrossRef {
            target: target.to_string(),
            line: 1,
            method: DetectionMethod::XmlCrossref,
        }
    }

    #[test]
    fn should_build_graph_from_crossrefs() {
        // Given: skill-a → skill-b → skill-c
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);
        crossrefs.insert("skill-b".to_string(), vec![test_crossref("skill-c")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);

        // Then
        assert_eq!(graph.name_to_node.len(), 3);
    }

    #[test]
    fn should_identify_root_skills() {
        // Given: skill-a → skill-b (skill-a is root)
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);

        // Then
        assert_eq!(graph.roots.len(), 1);
        assert!(graph.roots.contains(&"skill-a".to_string()));
    }

    #[test]
    fn should_identify_leaf_skills() {
        // Given: skill-a → skill-b (skill-b is leaf)
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);

        // Then
        assert_eq!(graph.leaves.len(), 1);
        assert!(graph.leaves.contains(&"skill-b".to_string()));
    }

    #[test]
    fn should_detect_clusters() {
        // Given: skill-a ↔ skill-b (circular reference, forms a cluster)
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);
        crossrefs.insert("skill-b".to_string(), vec![test_crossref("skill-a")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);

        // Then
        assert_eq!(graph.clusters.len(), 1);
        assert_eq!(graph.clusters[0].len(), 2);
    }

    #[test]
    fn should_generate_dot_output() {
        // Given
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);
        let dot = graph.to_dot();

        // Then
        assert!(dot.contains("digraph SkillGraph"));
        assert!(dot.contains("\"skill-a\" -> \"skill-b\""));
    }

    #[test]
    fn should_generate_json_output() {
        // Given
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);
        let json = graph.to_json();

        // Then
        assert!(json.contains("\"nodes\""));
        assert!(json.contains("\"edges\""));
        assert!(json.contains("skill-a"));
    }

    #[test]
    fn should_generate_mermaid_output() {
        // Given
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);
        let mermaid = graph.to_mermaid();

        // Then
        assert!(mermaid.contains("graph LR"));
        assert!(mermaid.contains("skill_a"));
        assert!(mermaid.contains("-->"));
    }

    #[test]
    fn should_deduplicate_edges() {
        // Given: skill-a references skill-b twice
        let mut crossrefs = HashMap::new();
        crossrefs.insert(
            "skill-a".to_string(),
            vec![test_crossref("skill-b"), test_crossref("skill-b")],
        );

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);
        let text = graph.to_text();

        // Then: skill-b should appear only once in the adjacency list
        let line = text.lines().find(|l| l.starts_with("skill-a:")).unwrap();
        assert_eq!(line, "skill-a: skill-b");
    }

    #[test]
    fn should_include_pipeline_edges() {
        // Given: skills with pipeline after/before declarations
        use crate::skill::frontmatter::{Frontmatter, PipelineStage};
        use std::path::PathBuf;

        let skills = vec![
            Skill {
                name: "skill-a".to_string(),
                path: PathBuf::from("/test/skill-a"),
                skill_file: PathBuf::from("/test/skill-a/SKILL.md"),
                frontmatter: Frontmatter {
                    name: "skill-a".to_string(),
                    description: "Test A".to_string(),
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
                    tags: None,
                    pipeline: Some({
                        let mut m = HashMap::new();
                        m.insert(
                            "test-pipeline".to_string(),
                            PipelineStage {
                                stage: "first".to_string(),
                                order: 1,
                                after: None,
                                before: Some(vec!["skill-b".to_string()]),
                            },
                        );
                        m
                    }),
                },
            },
            Skill {
                name: "skill-b".to_string(),
                path: PathBuf::from("/test/skill-b"),
                skill_file: PathBuf::from("/test/skill-b/SKILL.md"),
                frontmatter: Frontmatter {
                    name: "skill-b".to_string(),
                    description: "Test B".to_string(),
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
                    tags: None,
                    pipeline: Some({
                        let mut m = HashMap::new();
                        m.insert(
                            "test-pipeline".to_string(),
                            PipelineStage {
                                stage: "second".to_string(),
                                order: 2,
                                after: Some(vec!["skill-a".to_string()]),
                                before: None,
                            },
                        );
                        m
                    }),
                },
            },
        ];

        let crossrefs = HashMap::new(); // no crossrefs, only pipeline edges

        // When
        let graph = SkillGraph::from_skills(&crossrefs, &skills);
        let text = graph.to_text();

        // Then: pipeline edges create the dependency
        let line_b = text.lines().find(|l| l.starts_with("skill-b:")).unwrap();
        assert!(line_b.contains("skill-a"));
    }
}
