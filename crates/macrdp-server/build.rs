use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Candidate paths for Swift Concurrency runtime library
fn swift_lib_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 1. User-specified path (highest priority)
    if let Ok(path) = env::var("SWIFT_LIB_PATH") {
        candidates.push(PathBuf::from(path));
    }

    // 2. System Swift runtime (usually sufficient on macOS 12.3+)
    candidates.push(PathBuf::from("/usr/lib/swift"));

    // 3. Detect via xcode-select
    if let Ok(output) = Command::new("xcode-select").arg("-p").output() {
        if output.status.success() {
            let dev_path = String::from_utf8_lossy(&output.stdout).trim().to_string();

            if dev_path.contains("CommandLineTools") {
                // Command Line Tools installation
                candidates.push(PathBuf::from(format!(
                    "{dev_path}/usr/lib/swift-5.5/macosx"
                )));
                candidates.push(PathBuf::from(format!(
                    "{dev_path}/usr/lib/swift/macosx"
                )));
            } else {
                // Full Xcode installation
                candidates.push(PathBuf::from(format!(
                    "{dev_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift-5.5/macosx"
                )));
                candidates.push(PathBuf::from(format!(
                    "{dev_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx"
                )));
            }
        }
    }

    // 4. Common fallback paths
    candidates.push(PathBuf::from(
        "/Library/Developer/CommandLineTools/usr/lib/swift-5.5/macosx",
    ));
    candidates.push(PathBuf::from(
        "/Library/Developer/CommandLineTools/usr/lib/swift/macosx",
    ));

    candidates
}

fn main() {
    // Find and link Swift Concurrency runtime
    let target_lib = "libswift_Concurrency.dylib";

    // Check user-specified path first
    if let Ok(path) = env::var("SWIFT_LIB_PATH") {
        let p = PathBuf::from(&path);
        if p.join(target_lib).exists() {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", p.display());
            return;
        }
    }

    // If system path has it, no extra rpath needed (avoids duplicate loading warnings)
    let system_path = PathBuf::from("/usr/lib/swift");
    if system_path.join(target_lib).exists() {
        // /usr/lib/swift is already in the default search path, no rpath needed
        return;
    }

    // Otherwise search candidates
    for candidate in swift_lib_candidates() {
        if candidate.join(target_lib).exists() {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", candidate.display());
            return;
        }
    }

    println!("cargo:warning=Could not find {target_lib}. Set SWIFT_LIB_PATH env var to the directory containing it.");
}
