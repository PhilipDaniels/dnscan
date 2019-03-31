use std::path::{Path, PathBuf};

pub trait PathExtensions {
    // Returns the path as a str, or "" if it cannot be converted.
    fn as_str(&self) -> &str;
    // Returns the final filename component as a str, or "" if it cannot be converted.
    fn filename_as_str(&self) -> &str;
    // Returns the directory as a str, or "" if it cannot be converted.
    fn directory_as_str(&self) -> &str;
    // Returns the extension as a str, or "" if it cannot be converted.
    fn extension_as_str(&self) -> &str;
    fn eq_ignoring_case<P: AsRef<Path>>(&self, other: P) -> bool;
    fn is_same_dir<P: AsRef<Path>>(&self, other: P) -> bool;
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
                Some(s) => s,
            },
        }
    }

    fn directory_as_str(&self) -> &str {
        match self.parent() {
            None => "",
            Some(osstr) => match osstr.to_str() {
                None => "",
                Some(s) => s,
            },
        }
    }

    fn extension_as_str(&self) -> &str {
        //self.is_file() && self.extension().map_or(false, |s| s == "suo")

        match self.extension() {
            None => "",
            Some(osstr) => match osstr.to_str() {
                None => "",
                Some(s) => s,
            },
        }
    }

    /// Due to the awful situation on Windows, where paths embedded in project and solution files are
    /// often different in case to what is actually on disk, we perform most comparisons in a
    /// case-insensitive manner.
    fn eq_ignoring_case<P: AsRef<Path>>(&self, other: P) -> bool {
        unicase::eq_ascii(self.as_str(), other.as_ref().as_str())
    }

    fn is_same_dir<P: AsRef<Path>>(&self, other: P) -> bool {
        let p1 = self.parent().unwrap();
        let p2 = other.as_ref().parent().unwrap();
        p1.is_dir() && p2.is_dir() && p1.eq_ignoring_case(p2)
    }

    fn is_hidden_dir(&self) -> bool {
        self.is_dir() && self.filename_as_str().starts_with('.')
    }

    fn is_bin_or_obj_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && (
            unicase::eq_ascii(last_part, "obj")
            || unicase::eq_ascii(last_part, "bin")
        )
    }

    fn is_test_results_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, "TestResults")
    }

    fn is_packages_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, "packages")
    }

    fn is_node_modules_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, "node_modules")
    }

    fn is_git_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, ".git")
    }

    fn is_solution_info_file(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_file() && unicase::eq_ascii(last_part, "SolutionInfo.cs")
    }

    fn is_version_out_file(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_file() && unicase::eq_ascii(last_part, "VERSION.txt.out")
    }

    fn is_sln_file(&self) -> bool {
        let ext = self.extension_as_str();
        self.is_file() && unicase::eq_ascii(ext, "sln")
    }

    fn is_csproj_file(&self) -> bool {
        let ext = self.extension_as_str();
        self.is_file() && unicase::eq_ascii(ext, "csproj")
    }

    fn is_suo_file(&self) -> bool {
        let ext = self.extension_as_str();
        self.is_file() && unicase::eq_ascii(ext, "suo")
    }

    fn is_upgrade_log_file(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_file() && unicase::eq_ascii(last_part, "UpgradeLog.htm")
    }

    fn is_git_orig_file(&self) -> bool {
        let ext = self.extension_as_str();
        self.is_file() && unicase::eq_ascii(ext, "orig")
    }

    fn is_mef_cache_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, "ComponentModelCache")
    }

    fn is_jet_brains_cache_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, "SolutionCaches")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    pub fn eq_ignoring_case() {
        let p1 = PathBuf::from("");
        let p2 = PathBuf::from("");
        assert!(p1.eq_ignoring_case(p2));

        let p1 = PathBuf::from("a");
        let p2 = PathBuf::from("A");
        assert!(p1.eq_ignoring_case(p2));

        let p1 = PathBuf::from("a");
        let p2 = PathBuf::from("b");
        assert!(!p1.eq_ignoring_case(p2));

        let p1 = PathBuf::from(r"a\b\c");
        let p2 = PathBuf::from(r"A\B\c");
        assert!(p1.eq_ignoring_case(p2));
    }

    // #[test]
    // pub fn is_same_dir() {
    //     let p1 = PathBuf::from("a");
    //     let p2 = PathBuf::from("A");
    //     assert!(p1.is_same_dir(p2));

    //     let p1 = PathBuf::from(r"a\b");
    //     let p2 = PathBuf::from(r"A\b");
    //     assert!(p1.is_same_dir(p2));
    // }

    // #[test]
    // pub fn is_bin_or_obj_dir() {
    //     let p1 = PathBuf::from("");
    //     assert!(!p1.is_bin_or_obj_dir());

    //     let p1 = PathBuf::from("a");
    //     assert!(!p1.is_bin_or_obj_dir());

    //     let p1 = PathBuf::from("bin");
    //     assert!(!p1.is_bin_or_obj_dir());
    // }
}