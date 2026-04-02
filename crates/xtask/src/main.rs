use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

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
        Commands::BumpTo { version } => bump_version_to(version)?,
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

fn bump_version_to(version: String) -> Result<()> {
    println!("Bumping version to {}...", version);
    let root = project_root();

    // 1. Update crates/riffl-core/Cargo.toml
    update_cargo_toml(&root.join("crates/riffl-core/Cargo.toml"), &version, None)?;

    // 2. Update crates/riffl/Cargo.toml (version and dependency)
    update_cargo_toml(
        &root.join("crates/riffl/Cargo.toml"),
        &version,
        Some("riffl-core"),
    )?;

    // 3. Update flake.nix
    update_flake_nix(&root.join("flake.nix"), &version)?;

    println!("Version bumped to {}. Please verify and commit.", version);
    Ok(())
}

fn bump_version(part: VersionPart) -> Result<()> {
    println!("Bumping version {}...", part);

    let current_version =
        version_from_cargo_toml(&project_root().join("crates/riffl-core/Cargo.toml"))?;
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

    let root = project_root();

    // Update crates/riffl-core/Cargo.toml
    update_cargo_toml(
        &root.join("crates/riffl-core/Cargo.toml"),
        &target_version,
        None,
    )?;

    // Update crates/riffl/Cargo.toml (version and dependency)
    update_cargo_toml(
        &root.join("crates/riffl/Cargo.toml"),
        &target_version,
        Some("riffl-core"),
    )?;

    // Update flake.nix
    update_flake_nix(&root.join("flake.nix"), &target_version)?;

    println!(
        "Version bumped to {}. Please verify and commit.",
        target_version
    );
    Ok(())
}

fn update_cargo_toml(path: &Path, version: &str, dep_to_update: Option<&str>) -> Result<()> {
    let content = fs::read_to_string(path)?;
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

    fs::write(path, doc.to_string())?;
    Ok(())
}

fn update_flake_nix(path: &Path, version: &str) -> Result<()> {
    let content = fs::read_to_string(path)?;
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

    fs::write(path, new_content)?;
    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()))
        .ancestors()
        .nth(2)
        .expect("Failed to find project root")
        .to_path_buf()
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
