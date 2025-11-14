use std::path::Path;
use std::env;
use book_generator::utils::logging::cleanup_logs;

fn main() -> std::io::Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    // Default values
    let mut project_dir = String::from(".");
    let mut retention_days = 7;
    let mut keep_essential = true;
    
    // Parse arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--project" | "-p" => {
                if i + 1 < args.len() {
                    project_dir = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: Missing value for --project");
                    return Ok(());
                }
            },
            "--days" | "-d" => {
                if i + 1 < args.len() {
                    if let Ok(days) = args[i + 1].parse::<u64>() {
                        retention_days = days;
                        i += 2;
                    } else {
                        eprintln!("Error: Invalid value for --days, must be a positive number");
                        return Ok(());
                    }
                } else {
                    eprintln!("Error: Missing value for --days");
                    return Ok(());
                }
            },
            "--all" | "-a" => {
                keep_essential = false;
                i += 1;
            },
            "--help" | "-h" => {
                print_help();
                return Ok(());
            },
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_help();
                return Ok(());
            }
        }
    }
    
    // Check if the project directory exists
    let project_path = Path::new(&project_dir);
    if !project_path.exists() {
        eprintln!("Error: Project directory '{}' does not exist", project_dir);
        return Ok(());
    }
    
    // If the path is a directory but not an output directory, check if it contains output directories
    if project_path.is_dir() && !project_path.join("metadata.md").exists() {
        let output_dir = project_path.join("output");
        if output_dir.exists() && output_dir.is_dir() {
            println!("Cleaning up logs in all projects in the output directory...");
            
            let mut total_removed = 0;
            if let Ok(entries) = std::fs::read_dir(&output_dir) {
                for entry in entries.filter_map(|r| r.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        match cleanup_logs(&path, retention_days, keep_essential) {
                            Ok(count) => {
                                if count > 0 {
                                    println!("Cleaned up {} log files from {}", count, path.display());
                                    total_removed += count;
                                }
                            },
                            Err(e) => {
                                eprintln!("Error cleaning up logs in {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
            
            println!("Total log files removed: {}", total_removed);
            return Ok(());
        }
    }
    
    // Clean up logs in the specified project
    match cleanup_logs(project_path, retention_days, keep_essential) {
        Ok(count) => {
            println!("Cleaned up {} log files from {}", count, project_path.display());
        },
        Err(e) => {
            eprintln!("Error cleaning up logs: {}", e);
        }
    }
    
    Ok(())
}

fn print_help() {
    println!("Usage: cleanup_logs [OPTIONS]");
    println!("Clean up old log files from AuthorWorks projects");
    println!();
    println!("Options:");
    println!("  -p, --project PATH    Project directory (default: current directory)");
    println!("  -d, --days DAYS       Number of days to keep logs (default: 7)");
    println!("  -a, --all             Remove all logs, including essential ones");
    println!("  -h, --help            Print this help message");
    println!();
    println!("Examples:");
    println!("  cleanup_logs -p output/my-book -d 14    # Keep logs from the last 14 days");
    println!("  cleanup_logs -a                         # Remove all logs in current directory");
} 