use anyhow::Result;
use std::collections::HashMap;
use std::fs;

use crate::config::Config;
use crate::graph::SkillGraph;
use crate::skill;

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Dot,
    Text,
    Json,
    Mermaid,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dot" => Some(Self::Dot),
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            "mermaid" => Some(Self::Mermaid),
            _ => None,
        }
    }
}

/// Optional filter for graph command
pub enum GraphFilter {
    None,
    Pipeline(String),
    Tag(String),
}

pub fn graph(config: &Config, format: OutputFormat, filter: GraphFilter) -> Result<()> {
    use std::collections::HashSet;

    // Discover all skills
    let all_skills = skill::discover_all(&config.sources.skills)?;

    // Build set of known skill names for filtering
    let known_skills: HashSet<String> = all_skills.iter().map(|s| s.name.clone()).collect();

    // Extract cross-references
    let mut crossrefs = HashMap::new();
    for skill in &all_skills {
        let skill_md = skill.path.join("SKILL.md");
        let content = fs::read_to_string(&skill_md)?;
        let refs =
            skill::extract_references_with_filter(&content, &skill.name, Some(&known_skills));
        if !refs.is_empty() {
            crossrefs.insert(skill.name.clone(), refs);
        }
    }

    // Build the full graph (with pipeline edges and dedup)
    let full_graph = SkillGraph::from_skills(&crossrefs, &all_skills);

    // Apply filter
    let skill_graph = match &filter {
        GraphFilter::None => full_graph,
        GraphFilter::Pipeline(name) => {
            // Verify pipeline exists
            let exists = all_skills.iter().any(|s| {
                s.frontmatter
                    .pipeline
                    .as_ref()
                    .map(|p| p.contains_key(name.as_str()))
                    .unwrap_or(false)
            });
            if !exists {
                let mut available: Vec<String> = HashSet::<String>::new().into_iter().collect();
                for s in &all_skills {
                    if let Some(p) = &s.frontmatter.pipeline {
                        for name in p.keys() {
                            available.push(name.clone());
                        }
                    }
                }
                available.sort();
                available.dedup();
                anyhow::bail!(
                    "Pipeline '{}' not found. Available: {}",
                    name,
                    available.join(", ")
                );
            }
            full_graph.filter_pipeline(&all_skills, name)
        }
        GraphFilter::Tag(tag) => full_graph.filter_tag(&all_skills, tag),
    };

    // Output in requested format
    let output = match format {
        OutputFormat::Dot => skill_graph.to_dot(),
        OutputFormat::Text => skill_graph.to_text(),
        OutputFormat::Json => skill_graph.to_json(),
        OutputFormat::Mermaid => skill_graph.to_mermaid(),
    };

    println!("{}", output);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_output_format_case_insensitive() {
        // Given/When/Then
        assert!(matches!(
            OutputFormat::from_str("dot"),
            Some(OutputFormat::Dot)
        ));
        assert!(matches!(
            OutputFormat::from_str("DOT"),
            Some(OutputFormat::Dot)
        ));
        assert!(matches!(
            OutputFormat::from_str("text"),
            Some(OutputFormat::Text)
        ));
        assert!(matches!(
            OutputFormat::from_str("json"),
            Some(OutputFormat::Json)
        ));
        assert!(matches!(
            OutputFormat::from_str("mermaid"),
            Some(OutputFormat::Mermaid)
        ));
        assert!(OutputFormat::from_str("invalid").is_none());
    }
}
