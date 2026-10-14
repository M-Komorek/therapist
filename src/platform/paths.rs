use std::{ffi::OsString, path::PathBuf};

pub fn default_therapist_log_path(
    environment_variable: impl Fn(&str) -> Option<OsString>,
) -> PathBuf {
    state_home(environment_variable)
        .map(|state_home| state_home.join("therapist"))
        .unwrap_or_else(std::env::temp_dir)
        .join("therapist.log")
}

fn state_home(environment_variable: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    path_if_absolute(environment_variable("XDG_STATE_HOME")).or_else(|| {
        path_if_absolute(environment_variable("HOME")).map(|home| home.join(".local/state"))
    })
}

fn path_if_absolute(variable_value: Option<OsString>) -> Option<PathBuf> {
    variable_value
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_environment(
        variables: &'static [(&'static str, &'static str)],
    ) -> impl Fn(&str) -> Option<OsString> {
        move |name| {
            variables
                .iter()
                .find(|(variable_name, _)| *variable_name == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn given_xdg_state_home_when_finding_log_path_then_log_is_in_its_therapist_directory() {
        let environment =
            make_environment(&[("XDG_STATE_HOME", "/data/state"), ("HOME", "/home/user")]);

        assert_eq!(
            default_therapist_log_path(environment),
            PathBuf::from("/data/state/therapist/therapist.log")
        );
    }

    #[test]
    fn given_only_home_when_finding_log_path_then_log_is_in_local_state() {
        let environment = make_environment(&[("HOME", "/home/user")]);

        assert_eq!(
            default_therapist_log_path(environment),
            PathBuf::from("/home/user/.local/state/therapist/therapist.log")
        );
    }

    #[test]
    fn given_relative_xdg_state_home_when_finding_log_path_then_it_is_ignored() {
        let environment =
            make_environment(&[("XDG_STATE_HOME", "relative/state"), ("HOME", "/home/user")]);

        assert_eq!(
            default_therapist_log_path(environment),
            PathBuf::from("/home/user/.local/state/therapist/therapist.log")
        );
    }

    #[test]
    fn given_empty_xdg_state_home_when_finding_log_path_then_it_is_ignored() {
        let environment = make_environment(&[("XDG_STATE_HOME", ""), ("HOME", "/home/user")]);

        assert_eq!(
            default_therapist_log_path(environment),
            PathBuf::from("/home/user/.local/state/therapist/therapist.log")
        );
    }

    #[test]
    fn given_no_xdg_state_home_and_no_home_when_finding_log_path_then_log_is_in_temp_dir() {
        let environment = make_environment(&[]);

        assert_eq!(
            default_therapist_log_path(environment),
            std::env::temp_dir().join("therapist.log")
        );
    }
}
