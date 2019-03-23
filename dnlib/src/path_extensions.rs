use std::path::{Path, PathBuf};

pub trait PathExtensions {
    // Returns the path as a str, or "" if it cannot be converted.
    fn as_str(&self) -> &str;
    // Returns the final filename component as a str, or "" if it cannot be converted.
    fn filename_as_str(&self) -> &str;
    // Returns the directory as a str, or "" if it cannot be converted.
    fn directory_as_str(&self) -> &str;
    fn is_hidden_dir(&self) -> bool;
    fn is_bin_or_obj_dir(&self) -> bool;
    fn is_packages_dir(&self) -> bool;
    fn is_test_results_dir(&self) -> bool;
    fn is_node_modules_dir(&self) -> bool;
    fn is_git_dir(&self) -> bool;
    fn is_solution_info_file(&self) -> bool;
    fn is_version_out_file(&self) -> bool;
    fn is_sln_file(&self) -> bool;
    fn is_csproj_file(&self) -> bool;
    fn is_suo_file(&self) -> bool;
    fn is_upgrade_log_file(&self) -> bool;
    fn is_git_orig_file(&self) -> bool;
    fn is_mef_cache_dir(&self) -> bool;
    fn is_jet_brains_cache_dir(&self) -> bool;
}

impl PathExtensions for Path {
    fn as_str(&self) -> &str {
        self.to_str().unwrap_or_default()
    }

    fn filename_as_str(&self) -> &str {
        match self.file_name() {
            None => "",
            Some(osstr) => match osstr.to_str() {
                None => "",
                Some(s) => s
            }
        }
    }

    fn directory_as_str(&self) -> &str {
        match self.parent() {
            None => "",
            Some(osstr) => match osstr.to_str() {
                None => "",
                Some(s) => s
            }
        }
    }

    fn is_hidden_dir(&self) -> bool {
        self.is_dir() && self.filename_as_str().starts_with(".")
    }

    fn is_bin_or_obj_dir(&self) -> bool {
        self.is_dir() && (self.ends_with("obj") || self.ends_with("bin"))
    }

    fn is_test_results_dir(&self) -> bool {
        self.is_dir() && (self.ends_with("TestResults") || self.ends_with("testresults"))
    }

    fn is_packages_dir(&self) -> bool {
        self.is_dir() && (self.ends_with("packages") || self.ends_with("Packages"))
    }

    fn is_node_modules_dir(&self) -> bool {
        self.is_dir() && self.ends_with("node_modules")
    }

    fn is_git_dir(&self) -> bool {
        self.is_dir() && self.ends_with(".git")
    }

    fn is_solution_info_file(&self) -> bool {
        self.is_file() && (self.ends_with("SolutionInfo.cs") || self.ends_with("solutioninfo.cs"))
    }

    fn is_version_out_file(&self) -> bool {
        self.is_file() && (self.ends_with("VERSION.txt.out") || self.ends_with("version.txt.out"))
    }

    fn is_sln_file(&self) -> bool {
        self.is_file() && self.extension().map_or(false, |s| s == "sln")
    }

    fn is_csproj_file(&self) -> bool {
        self.is_file() && self.extension().map_or(false, |s| s == "csproj")
    }

    fn is_suo_file(&self) -> bool {
        self.is_file() && self.extension().map_or(false, |s| s == "suo")
    }

    fn is_upgrade_log_file(&self) -> bool {
        self.is_file() && (self.ends_with("UpgradeLog.htm") || self.ends_with("upgradelog.htm"))
    }

    fn is_git_orig_file(&self) -> bool {
        self.is_file() && self.extension().map_or(false, |s| s == "orig")
    }

    fn is_mef_cache_dir(&self) -> bool {
        self.is_dir() && self.ends_with("ComponentModelCache")
    }

    fn is_jet_brains_cache_dir(&self) -> bool {
        self.is_dir() && self.ends_with("SolutionCaches")
    }
}

/// Return the home directory. Ok to panic if we cannot determine it.
/// Note that we do this lazily (not all code paths call this function).
pub fn home_dir() -> PathBuf {
    dirs::home_dir().expect("Cannot determine your home directory")
}

pub fn make_path_under_home_dir(sub_path: &str) -> PathBuf {
    let mut p = home_dir();
    p.push(sub_path);
    p
}