use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

#[derive(Debug)]
struct CrateMetadta {
    name: &'static str,
    /// Path releative to the root of the workspace
    path: &'static str,
}

const RIFFLE_CORE_CRATE: CrateMetadta = CrateMetadta {
    name: "riffl-core",
    path: "crates/riffl-core",
};

const RIFFL_TUI_CRATE: CrateMetadta = CrateMetadta {
    name: "riffl-tui",
    path: "crates/riffl-tui",
};
const FLAKE_NIX: &str = "flake.nix";

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build and automation tasks for Riffl", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Format all crates in the workspace
    Fmt,
    /// Run full CI-style checks (formatting, clippy, tests)
    Check,
    /// Run the 'riffl' TUI application
    Run {
        /// Additional arguments to pass to the binary
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run all tests in the workspace
    Test {
        /// Additional arguments to pass to cargo test
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Bump the version across all crates and flake.nix to a specific version
    BumpTo {
        /// The new version string (e.g., 0.1.4)
        version: String,
    },
    /// Increment a specific part of the version (major, minor, or patch)
    Bump {
        /// The part of the version to increment
        #[arg(value_enum)]
        part: VersionPart,
    },
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum VersionPart {
    Major,
    Minor,
    Patch,
}

impl std::fmt::Display for VersionPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionPart::Major => write!(f, "major"),
            VersionPart::Minor => write!(f, "minor"),
            VersionPart::Patch => write!(f, "patch"),
        }
    }
}

#[derive(Debug, Clone)]
struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
}

impl FromStr for SemVer {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow!("Invalid version string: {}. Expected x.y.z", s));
        }
        Ok(SemVer {
            major: parts[0].parse().context("Failed to parse major version")?,
            minor: parts[1].parse().context("Failed to parse minor version")?,
            patch: parts[2].parse().context("Failed to parse patch version")?,
        })
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Fmt => run_fmt()?,
        Commands::Check => run_check()?,
        Commands::Run { args } => run_riffl(args)?,
        Commands::Test { args } => run_tests(args)?,
        Commands::BumpTo { version } => bump_version_to(&version)?,
        Commands::Bump { part } => bump_version(part)?,
    }

    Ok(())
}

fn run_fmt() -> Result<()> {
    println!("Running cargo fmt...");
    let status = Command::new("cargo")
        .args(["fmt", "--all"])
        .status()
        .context("Failed to run cargo fmt")?;

    if !status.success() {
        return Err(anyhow!("cargo fmt failed"));
    }
    Ok(())
}

fn run_check() -> Result<()> {
    println!("Running CI-style checks...");

    println!("1. Checking formatting...");
    let status = Command::new("cargo")
        .args(["fmt", "--all", "--", "--check"])
        .status()
        .context("Failed to run cargo fmt --check")?;
    if !status.success() {
        return Err(anyhow!(
            "Formatting check failed. Run 'cargo xtask fmt' to fix."
        ));
    }

    println!("2. Running clippy...");
    let status = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ])
        .status()
        .context("Failed to run cargo clippy")?;
    if !status.success() {
        return Err(anyhow!("Clippy check failed"));
    }

    println!("3. Running tests...");
    run_tests(vec![])?;

    println!("All checks passed!");
    Ok(())
}

fn run_riffl(args: Vec<String>) -> Result<()> {
    println!("Starting riffl...");
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--bin", "riffl"]);
    if !args.is_empty() {
        cmd.arg("--");
        cmd.args(args);
    }

    let status = cmd.status().context("Failed to run riffl")?;
    if !status.success() {
        return Err(anyhow!("riffl exited with non-zero status"));
    }
    Ok(())
}

fn run_tests(args: Vec<String>) -> Result<()> {
    println!("Running tests...");
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--workspace"]);
    if !args.is_empty() {
        cmd.args(args);
    }

    let status = cmd.status().context("Failed to run tests")?;
    if !status.success() {
        return Err(anyhow!("Tests failed"));
    }
    Ok(())
}

fn bump_version_to(version: &str) -> Result<()> {
    println!("Bumping version to {}...", version);
    let root = project_root();

    let core_manifest = root.join(RIFFLE_CORE_CRATE.path).join("Cargo.toml");
    let tui_manifest = root.join(RIFFL_TUI_CRATE.path).join("Cargo.toml");
    let flake_nix = root.join(FLAKE_NIX);

    // Read all files into memory
    let core_content = fs::read_to_string(&core_manifest)
        .with_context(|| format!("Failed to read {}", core_manifest.display()))?;
    let tui_content = fs::read_to_string(&tui_manifest)
        .with_context(|| format!("Failed to read {}", tui_manifest.display()))?;
    let flake_content = fs::read_to_string(&flake_nix)
        .with_context(|| format!("Failed to read {}", flake_nix.display()))?;

    // Prepare new contents (this handles parsing and logic errors)
    let core_new = get_updated_cargo_toml(&core_content, version, None)?;
    let tui_new = get_updated_cargo_toml(&tui_content, version, Some(RIFFLE_CORE_CRATE.name))?;
    let flake_new = get_updated_flake_nix(&flake_content, version)?;

    let updates = [
        (&core_manifest, core_new, core_content),
        (&tui_manifest, tui_new, tui_content),
        (&flake_nix, flake_new, flake_content),
    ];

    // Write updates with rollback on failure
    for (written_count, (path, new_content, _)) in updates.iter().enumerate() {
        println!("Writing to {}...", path.display());
        if let Err(e) = fs::write(path, new_content) {
            println!(
                "Error writing to {}: {}. Rolling back changes...",
                path.display(),
                e
            );

            // Rollback previous writes
            for (r_path, _, r_original) in updates.iter().take(written_count) {
                if let Err(rollback_err) = fs::write(r_path, r_original) {
                    eprintln!(
                        "CRITICAL: Rollback failed for {}: {}",
                        r_path.display(),
                        rollback_err
                    );
                }
            }
            return Err(anyhow!("Failed to update {}: {}", path.display(), e));
        }
    }

    println!("Version bumped to {}. Please verify and commit.", version);
    Ok(())
}

fn bump_version(part: VersionPart) -> Result<()> {
    println!("Bumping version {}...", part);

    let current_version = version_from_cargo_toml(
        &project_root()
            .join(RIFFLE_CORE_CRATE.path)
            .join("Cargo.toml"),
    )?;
    let target_version = match part {
        VersionPart::Major => format!("{}.0.0", current_version.major + 1),
        VersionPart::Minor => format!("{}.{}.0", current_version.major, current_version.minor + 1),
        VersionPart::Patch => format!(
            "{}.{}.{}",
            current_version.major,
            current_version.minor,
            current_version.patch + 1
        ),
    };

    bump_version_to(&target_version)
}

fn get_updated_cargo_toml(
    content: &str,
    version: &str,
    dep_to_update: Option<&str>,
) -> Result<String> {
    let mut doc = content.parse::<toml_edit::DocumentMut>()?;

    // Update [package] version
    if let Some(package) = doc.get_mut("package").and_then(|p| p.as_table_mut()) {
        package.insert("version", toml_edit::value(version));
    }

    // Update dependency version if requested
    if let Some(dep_name) = dep_to_update {
        if let Some(deps) = doc.get_mut("dependencies").and_then(|d| d.as_table_mut()) {
            if let Some(dep) = deps.get_mut(dep_name) {
                if let Some(table) = dep.as_inline_table_mut() {
                    table.insert("version", version.into());
                } else if let Some(table) = dep.as_table_mut() {
                    table.insert("version", toml_edit::value(version));
                }
            }
        }
    }

    Ok(doc.to_string())
}

fn get_updated_flake_nix(content: &str, version: &str) -> Result<String> {
    // flake.nix is not TOML, using simple regex-like replacement
    let new_content = content
        .lines()
        .map(|line| {
            if line.trim().starts_with("version = \"") && line.trim().ends_with("\";") {
                let indent = line.find('v').unwrap_or(0);
                format!("{:indent$}version = \"{}\";", "", version, indent = indent)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(new_content)
}

fn project_root() -> PathBuf {
    riffl_core::metadata::find_project_root().expect("Failed to find project root")
}

fn version_from_cargo_toml(path: &Path) -> Result<SemVer> {
    let content = fs::read_to_string(path).context("Failed to read Cargo.toml")?;
    let doc = content.parse::<toml_edit::DocumentMut>()?;

    if let Some(package) = doc.get("package").and_then(|p| p.as_table()) {
        if let Some(version) = package.get("version").and_then(|v| v.as_str()) {
            version.parse::<SemVer>()
        } else {
            Err(anyhow!("Failed to find version in Cargo.toml"))
        }
    } else {
        Err(anyhow!("Failed to find package in Cargo.toml"))
    }
}
