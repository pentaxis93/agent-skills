//! List command implementation

use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::fs;

use crate::config::Config;
use crate::skill;

pub enum ListMode {
    Default,
    Groups,
    Refs(String),
    Missing,
}

/// List enabled skills per scope
pub fn list(config: &Config, mode: ListMode) -> Result<()> {
    match mode {
        ListMode::Default => list_default(config),
        ListMode::Groups => list_groups(config),
        ListMode::Refs(skill_name) => list_refs(config, &skill_name),
        ListMode::Missing => list_missing(config),
    }
}

fn list_default(config: &Config) -> Result<()> {
    // Discover all available skills
    let skills = skill::discover_all(&config.sources.skills)?;
    let skill_map = skill::build_skill_map(skills);

    // List global skills
    println!("{}", "--- Global scope ---".cyan().bold());
    println!("Skills: {}", config.global.skills.len());
    for skill_name in &config.global.skills {
        if let Some(skill) = skill_map.get(skill_name) {
            println!(
                "  {} {} ({})",
                "✓".green(),
                skill_name,
                skill.path.display().to_string().dimmed()
            );
        } else {
            println!("  {} {} {}", "✗".red(), skill_name, "(not found)".red());
        }
    }

    // List project skills
    for (project_path, project_config) in &config.projects {
        println!();
        println!(
            "{} {}",
            "--- Project:".cyan().bold(),
            project_path.display()
        );

        let mut all_skills = Vec::new();

        // Add global skills if inherited
        if project_config.inherit {
            all_skills.extend(config.global.skills.clone());
        }

        // Add project-specific skills
        all_skills.extend(project_config.skills.clone());

        // Deduplicate
        all_skills.sort();
        all_skills.dedup();

        println!(
            "Skills: {} (inherit: {})",
            all_skills.len(),
            if project_config.inherit {
                "true"
            } else {
                "false"
            }
        );

        for skill_name in &all_skills {
            if let Some(skill) = skill_map.get(skill_name) {
                let source = if config.global.skills.contains(skill_name) {
                    "global".dimmed()
                } else {
                    "project".dimmed()
                };
                println!(
                    "  {} {} ({}, {})",
                    "✓".green(),
                    skill_name,
                    source,
                    skill.path.display().to_string().dimmed()
                );
            } else {
                println!("  {} {} {}", "✗".red(), skill_name, "(not found)".red());
            }
        }
    }

    Ok(())
}

#[cfg(feature = "graph")]
fn list_groups(config: &Config) -> Result<()> {
    use crate::graph::SkillGraph;

    let skills = skill::discover_all(&config.sources.skills)?;
    let mut crossrefs = HashMap::new();

    for skill in &skills {
        let skill_md = skill.path.join("SKILL.md");
        let content = fs::read_to_string(&skill_md)?;
        let refs = skill::extract_references(&content, &skill.name);
        if !refs.is_empty() {
            crossrefs.insert(skill.name.clone(), refs);
        }
    }

    let graph = SkillGraph::from_crossrefs(&crossrefs);

    println!("{}", "--- Skills by cluster ---".cyan().bold());

    if graph.clusters.is_empty() {
        println!(
            "{}",
            "No clusters detected (no circular references)".dimmed()
        );
        println!("\nShowing all skills:");
        let mut all_names: Vec<_> = skills.iter().map(|s| &s.name).collect();
        all_names.sort();
        for name in all_names {
            println!("  • {}", name);
        }
    } else {
        for (i, cluster) in graph.clusters.iter().enumerate() {
            println!(
                "\n{} {}",
                format!("Cluster {}:", i + 1).yellow().bold(),
                format!("({} skills)", cluster.len()).dimmed()
            );
            for skill in cluster {
                println!("  • {}", skill);
            }
        }

        // Show unclustered skills
        let clustered: HashSet<_> = graph.clusters.iter().flat_map(|c| c.iter()).collect();
        let unclustered: Vec<_> = skills
            .iter()
            .filter(|s| !clustered.contains(&&s.name))
            .map(|s| &s.name)
            .collect();

        if !unclustered.is_empty() {
            println!("\n{}", "Unclustered skills:".dimmed());
            for skill in unclustered {
                println!("  • {}", skill);
            }
        }
    }

    Ok(())
}

#[cfg(not(feature = "graph"))]
fn list_groups(config: &Config) -> Result<()> {
    let skills = skill::discover_all(&config.sources.skills)?;

    println!(
        "{}",
        "--- Skills (cluster detection unavailable) ---"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "Note: Install with --features graph for cluster detection\n".yellow()
    );

    let mut all_names: Vec<_> = skills.iter().map(|s| &s.name).collect();
    all_names.sort();
    for name in all_names {
        println!("  • {}", name);
    }

    Ok(())
}

fn list_refs(config: &Config, skill_name: &str) -> Result<()> {
    let skills = skill::discover_all(&config.sources.skills)?;
    let skill_map = skill::build_skill_map(skills.clone());

    // Check if skill exists
    if !skill_map.contains_key(skill_name) {
        anyhow::bail!("Skill '{}' not found in any source", skill_name);
    }

    // Extract all cross-references
    let mut crossrefs: HashMap<String, Vec<skill::CrossRef>> = HashMap::new();
    for skill in &skills {
        let skill_md = skill.path.join("SKILL.md");
        let content = fs::read_to_string(&skill_md)?;
        let refs = skill::extract_references(&content, &skill.name);
        if !refs.is_empty() {
            crossrefs.insert(skill.name.clone(), refs);
        }
    }

    // Find outgoing references (skills this skill references)
    let outgoing: Vec<String> = crossrefs
        .get(skill_name)
        .map(|refs| refs.iter().map(|r| r.target.clone()).collect())
        .unwrap_or_default();

    // Find incoming references (skills that reference this skill)
    let incoming: Vec<String> = crossrefs
        .iter()
        .filter(|(_, refs)| refs.iter().any(|r| r.target == skill_name))
        .map(|(name, _)| name.clone())
        .collect();

    println!(
        "{} {}",
        "--- References for".cyan().bold(),
        skill_name.cyan().bold()
    );

    println!("\n{} ({})", "Outgoing:".yellow(), outgoing.len());
    if outgoing.is_empty() {
        println!("  {}", "(none)".dimmed());
    } else {
        for target in &outgoing {
            println!("  → {}", target);
        }
    }

    println!("\n{} ({})", "Incoming:".green(), incoming.len());
    if incoming.is_empty() {
        println!("  {}", "(none)".dimmed());
    } else {
        for source in &incoming {
            println!("  ← {}", source);
        }
    }

    Ok(())
}

fn list_missing(config: &Config) -> Result<()> {
    let skills = skill::discover_all(&config.sources.skills)?;
    let skill_map = skill::build_skill_map(skills.clone());

    // Extract all cross-references
    let mut all_referenced: HashSet<String> = HashSet::new();
    for skill in &skills {
        let skill_md = skill.path.join("SKILL.md");
        let content = fs::read_to_string(&skill_md)
            .context(format!("Failed to read {}", skill_md.display()))?;
        let refs = skill::extract_references(&content, &skill.name);
        for r in refs {
            all_referenced.insert(r.target);
        }
    }

    // Find dangling references
    let mut missing: Vec<String> = all_referenced
        .iter()
        .filter(|name| !skill_map.contains_key(*name))
        .cloned()
        .collect();
    missing.sort();

    println!(
        "{}",
        "--- Missing skills (dangling references) ---".cyan().bold()
    );

    if missing.is_empty() {
        println!("{}", "No missing skills found.".green());
    } else {
        println!(
            "{} missing skills referenced:\n",
            missing.len().to_string().red().bold()
        );
        for name in &missing {
            println!("  {} {}", "✗".red(), name.red());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Global, Project, Sources};
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skills(temp: &TempDir) {
        let skills_dir = temp.path().join("skills");

        let test_skill_dir = skills_dir.join("test-skill");
        fs::create_dir_all(&test_skill_dir).unwrap();
        fs::write(
            test_skill_dir.join("SKILL.md"),
            "---\nname: test-skill\ndescription: Test skill\n---\n",
        )
        .unwrap();

        let another_skill_dir = skills_dir.join("another-skill");
        fs::create_dir_all(&another_skill_dir).unwrap();
        fs::write(
            another_skill_dir.join("SKILL.md"),
            "---\nname: another-skill\ndescription: Another test skill\n---\n\n<crossrefs>\n  <see ref=\"test-skill\">Related</see>\n</crossrefs>",
        )
        .unwrap();
    }

    #[test]
    fn should_list_default_mode() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec!["test-skill".to_string()],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Default);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_list_refs_for_skill() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Refs("test-skill".to_string()));

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_error_when_skill_not_found_for_refs() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Refs("nonexistent".to_string()));

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn should_list_missing_skills() {
        // Given
        let temp = TempDir::new().unwrap();
        let skills_dir = temp.path().join("skills");
        let skill_dir = skills_dir.join("referrer");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: referrer\ndescription: Refs nonexistent\n---\n\n<crossrefs>\n  <see ref=\"nonexistent\">Missing</see>\n</crossrefs>",
        )
        .unwrap();

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Missing);

        // Then
        assert!(result.is_ok());
    }
}
