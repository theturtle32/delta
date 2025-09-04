use std::env;
use std::path::Path;

const COLORTERM: &str = "COLORTERM";
const BAT_THEME: &str = "BAT_THEME";
const GIT_CONFIG_PARAMETERS: &str = "GIT_CONFIG_PARAMETERS";
const GIT_PREFIX: &str = "GIT_PREFIX";
const DELTA_FEATURES: &str = "DELTA_FEATURES";
const DELTA_NAVIGATE: &str = "DELTA_NAVIGATE";
const DELTA_EXPERIMENTAL_MAX_LINE_DISTANCE_FOR_NAIVELY_PAIRED_LINES: &str =
    "DELTA_EXPERIMENTAL_MAX_LINE_DISTANCE_FOR_NAIVELY_PAIRED_LINES";
const DELTA_PAGER: &str = "DELTA_PAGER";

#[derive(Default, Clone)]
pub struct DeltaEnv {
    pub bat_theme: Option<String>,
    pub colorterm: Option<String>,
    pub current_dir: Option<std::path::PathBuf>,
    pub experimental_max_line_distance_for_naively_paired_lines: Option<String>,
    pub features: Option<String>,
    pub git_config_parameters: Option<String>,
    pub git_prefix: Option<String>,
    pub hostname: Option<String>,
    pub navigate: Option<String>,
    pub pagers: (Option<String>, Option<String>),
}

impl DeltaEnv {
    /// Create a structure with current environment variable
    pub fn init() -> Self {
        let bat_theme = env::var(BAT_THEME).ok();
        let colorterm = env::var(COLORTERM).ok();
        let experimental_max_line_distance_for_naively_paired_lines =
            env::var(DELTA_EXPERIMENTAL_MAX_LINE_DISTANCE_FOR_NAIVELY_PAIRED_LINES).ok();
        let features = env::var(DELTA_FEATURES).ok();
        let git_config_parameters = env::var(GIT_CONFIG_PARAMETERS).ok();
        let git_prefix = env::var(GIT_PREFIX).ok();
        let hostname = hostname();
        let navigate = env::var(DELTA_NAVIGATE).ok();

        let current_dir = env::current_dir().ok();
        let pagers = (
            env::var(DELTA_PAGER).ok(),
            // Reimplement bat's pager detection logic to preserve full PAGER commands.
            // This fixes the bug where bat::config::get_pager_executable(None) was stripping
            // arguments from complex PAGER commands like '/bin/sh -c "head -10000 | cat"'.
            // We can't use bat::pager::get_pager directly because the pager module is private.
            get_pager_from_env(),
        );

        Self {
            bat_theme,
            colorterm,
            current_dir,
            experimental_max_line_distance_for_naively_paired_lines,
            features,
            git_config_parameters,
            git_prefix,
            hostname,
            navigate,
            pagers,
        }
    }
}

fn hostname() -> Option<String> {
    grep_cli::hostname().ok()?.to_str().map(|s| s.to_string())
}

#[cfg(test)]
pub mod tests {
    use super::DeltaEnv;
    use lazy_static::lazy_static;
    use std::env;
    use std::sync::{Arc, Mutex};

    lazy_static! {
        static ref ENV_ACCESS: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    }

    #[test]
    fn test_env_parsing() {
        let _guard = ENV_ACCESS.lock().unwrap();
        let feature = "Awesome Feature";
        env::set_var("DELTA_FEATURES", feature);
        let env = DeltaEnv::init();
        drop(_guard);
        assert_eq!(env.features, Some(feature.into()));
        // otherwise `current_dir` is not used in the test cfg:
        assert_eq!(env.current_dir, env::current_dir().ok());
    }

    #[test]
    fn test_env_parsing_with_pager_set_to_bat() {
        let _guard = ENV_ACCESS.lock().unwrap();
        env::set_var("PAGER", "bat");
        let env = DeltaEnv::init();
        drop(_guard);
        assert_eq!(
            env.pagers.1,
            Some("bat".into()),
            "Expected env.pagers.1 == Some(bat) but was {:?}",
            env.pagers.1
        );
    }

    #[test]
    fn test_env_parsing_with_pager_set_to_more() {
        let _guard = ENV_ACCESS.lock().unwrap();
        env::set_var("PAGER", "more");
        let env = DeltaEnv::init();
        drop(_guard);
        assert_eq!(env.pagers.1, Some("less".into()));
    }

    #[test]
    fn test_env_parsing_with_pager_set_to_most() {
        let _guard = ENV_ACCESS.lock().unwrap();
        env::set_var("PAGER", "most");
        let env = DeltaEnv::init();
        drop(_guard);
        assert_eq!(env.pagers.1, Some("less".into()));
    }

    #[test]
    fn test_env_parsing_with_complex_shell_pager_command() {
        // This test verifies the core bug fix: complex PAGER commands with arguments
        // should be preserved, not stripped down to just the executable path.
        let _guard = ENV_ACCESS.lock().unwrap();
        env::set_var("PAGER", "/bin/sh -c \"head -10000 | cat\"");
        let env = DeltaEnv::init();
        drop(_guard);
        assert_eq!(
            env.pagers.1,
            Some("/bin/sh -c \"head -10000 | cat\"".into()),
            "Complex shell pager command should be preserved with arguments"
        );
    }

    #[test]
    fn test_env_parsing_with_simple_shell_pager_command() {
        let _guard = ENV_ACCESS.lock().unwrap();
        env::set_var("PAGER", "/bin/sh -c \"cat\"");
        let env = DeltaEnv::init();
        drop(_guard);
        assert_eq!(
            env.pagers.1,
            Some("/bin/sh -c \"cat\"".into()),
            "Simple shell pager command should be preserved with arguments"
        );
    }

    #[test]
    fn test_env_parsing_with_pager_arguments_preserved() {
        // Test that pager commands with various argument styles are preserved
        let _guard = ENV_ACCESS.lock().unwrap();
        env::set_var("PAGER", "less -R -F -X");
        let env = DeltaEnv::init();
        drop(_guard);
        assert_eq!(
            env.pagers.1,
            Some("less -R -F -X".into()),
            "Pager arguments should be preserved"
        );
    }

    #[test]
    fn test_env_parsing_delta_pager_takes_precedence() {
        // Test that DELTA_PAGER takes precedence over PAGER
        let _guard = ENV_ACCESS.lock().unwrap();
        env::set_var("PAGER", "cat");
        env::set_var("DELTA_PAGER", "/bin/sh -c \"head -1 | cat\"");
        let env = DeltaEnv::init();
        drop(_guard);
        assert_eq!(
            env.pagers.0,
            Some("/bin/sh -c \"head -1 | cat\"".into()),
            "DELTA_PAGER should be preserved exactly as set"
        );
        assert_eq!(
            env.pagers.1,
            Some("cat".into()),
            "PAGER should also be preserved for fallback"
        );
    }
}

/// Get pager from environment variables using bat's logic.
/// This reimplements bat's pager::get_pager function to preserve full PAGER commands
/// including arguments, while still handling problematic pagers properly.
fn get_pager_from_env() -> Option<String> {
    let bat_pager = env::var("BAT_PAGER");
    let pager = env::var("PAGER");

    let (cmd, from_pager_env) = match (&bat_pager, &pager) {
        (Ok(bat_pager), _) => (bat_pager.as_str(), false),
        (_, Ok(pager)) => (pager.as_str(), true),
        _ => ("less", false),
    };

    // Parse the command using shell_words to split into binary and arguments
    if let Ok(parts) = shell_words::split(cmd) {
        if let Some((bin, args)) = parts.split_first() {
            // Determine what kind of pager this is
            let pager_bin = Path::new(bin).file_stem();
            let current_bin = env::args_os().next();

            let is_current_bin_pager = current_bin
                .map(|s| Path::new(&s).file_stem() == pager_bin)
                .unwrap_or(false);

            let is_problematic_pager = if from_pager_env {
                // Only replace problematic pagers when they come from PAGER env var
                match pager_bin.map(|s| s.to_string_lossy()).as_deref() {
                    Some("more") | Some("most") => true,
                    _ if is_current_bin_pager => true, // Prevent recursion
                    _ => false,
                }
            } else {
                false
            };

            if is_problematic_pager {
                // Replace problematic pagers with "less"
                Some("less".to_string())
            } else {
                // Preserve the original command string unmodified to maintain proper quoting
                Some(cmd.to_string())
            }
        } else {
            Some("less".to_string())
        }
    } else {
        Some("less".to_string())
    }
}
