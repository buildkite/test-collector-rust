//! # run_env
//!
//! Runtime CI environment detection and serialisation.

use std::env;
use uuid::Uuid;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static COLLECTOR_NAME: &str = env!("CARGO_PKG_NAME");

/// # RuntimeEnvironment
///
/// A data structure containing information about the detected CI environment.
#[derive(serde::Serialize, Debug, PartialEq, Clone)]
pub struct RuntimeEnvironment {
    ci: String,
    key: String,
    number: Option<String>,
    job_id: Option<String>,
    branch: Option<String>,
    commit_sha: Option<String>,
    message: Option<String>,
    url: Option<String>,
    collector: String,
    version: String,
}

impl RuntimeEnvironment {
    /// Detect the runtime environment
    ///
    /// Attempts to detect the environment based on the environment variables
    /// which are present.  Returns `None` on failure.
    pub fn detect() -> Option<RuntimeEnvironment> {
        buildkite_env()
            .or_else(github_actions_env)
            .or_else(circle_ci_env)
            .or_else(generic_env)
    }

    #[cfg(test)]
    pub fn generic() -> RuntimeEnvironment {
        RuntimeEnvironment {
            ci: "generic".to_string(),
            key: Uuid::new_v4().to_string(),
            number: None,
            job_id: None,
            branch: None,
            commit_sha: None,
            message: None,
            url: None,
            collector: format!("rust-{}", COLLECTOR_NAME.to_string()),
            version: VERSION.to_string(),
        }
    }
}

fn buildkite_env() -> Option<RuntimeEnvironment> {
    let build_id = maybe_var("BUILDKITE_BUILD_ID")?;

    Some(RuntimeEnvironment {
        ci: "buildkite".to_string(),
        key: build_id,
        url: maybe_var("BUILDKITE_BUILD_URL"),
        branch: maybe_var("BUILDKITE_BRANCH"),
        commit_sha: maybe_var("BUILDKITE_COMMIT"),
        number: maybe_var("BUILDKITE_BUILD_NUMBER"),
        job_id: maybe_var("BUILDKITE_JOB_ID"),
        message: maybe_var("BUILDKITE_MESSAGE"),
        collector: format!("rust-{}", COLLECTOR_NAME.to_string()),
        version: VERSION.to_string(),
    })
}

fn github_actions_env() -> Option<RuntimeEnvironment> {
    let action = maybe_var("GITHUB_ACTION")?;
    let run_number = maybe_var("GITHUB_RUN_NUMBER")?;
    let run_attempt = maybe_var("GITHUB_RUN_ATTEMPT")?;

    Some(RuntimeEnvironment {
        ci: "github_actions".to_string(),
        key: format!("{}-{}-{}", action, run_number, run_attempt),
        url: maybe_var("GITHUB_REPOSITORY")
            .zip(maybe_var("GITHUB_RUN_ID"))
            .map(|(repo, run_id)| format!("https://github.com/{}/actions/runs/{}", repo, run_id)),
        branch: maybe_var("GITHUB_REF"),
        commit_sha: maybe_var("GITHUB_SHA"),
        number: Some(run_number),
        job_id: None,
        message: None,
        collector: format!("rust-{}", COLLECTOR_NAME.to_string()),
        version: VERSION.to_string(),
    })
}

fn circle_ci_env() -> Option<RuntimeEnvironment> {
    let build_num = maybe_var("CIRCLE_BUILD_NUM")?;
    let workflow_id = maybe_var("CIRCLE_WORKFLOW_ID")?;

    Some(RuntimeEnvironment {
        ci: "circleci".to_string(),
        key: format!("{}-{}", workflow_id, build_num),
        url: maybe_var("CIRCLE_BUILD_URL"),
        branch: maybe_var("CIRCLE_BRANCH"),
        commit_sha: maybe_var("CIRCLE_SHA1"),
        number: Some(build_num),
        job_id: None,
        message: None,
        collector: format!("rust-{}", COLLECTOR_NAME.to_string()),
        version: VERSION.to_string(),
    })
}

fn generic_env() -> Option<RuntimeEnvironment> {
    maybe_var("CI")?;

    Some(RuntimeEnvironment {
        ci: "generic".to_string(),
        key: Uuid::new_v4().to_string(),
        number: None,
        job_id: None,
        branch: None,
        commit_sha: None,
        message: None,
        url: None,
        collector: format!("rust-{}", COLLECTOR_NAME.to_string()),
        version: VERSION.to_string(),
    })
}

fn maybe_var(key: &str) -> Option<String> {
    env::var(key).ok()
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::Rng;
    use std::collections::HashMap;

    #[test]
    #[serial]
    fn detects_buildkite_environment() {
        let mut rng = rand::thread_rng();

        with_clean_environment(|| {
            let build_id = Uuid::new_v4().to_string();
            let url = format!("https://example.test/{}", build_id);
            let branch = "marty".to_string();
            let commit_sha = Uuid::new_v4().to_string().replace('-', "");
            let number = rng.gen_range(0..999).to_string();
            let job_id = rng.gen_range(0..999).to_string();
            let message = "Be excellent to each other".to_string();

            env::set_var("BUILDKITE_BUILD_ID", &build_id);
            env::set_var("BUILDKITE_BUILD_URL", &url);
            env::set_var("BUILDKITE_BRANCH", &branch);
            env::set_var("BUILDKITE_COMMIT", &commit_sha);
            env::set_var("BUILDKITE_BUILD_NUMBER", &number);
            env::set_var("BUILDKITE_JOB_ID", &job_id);
            env::set_var("BUILDKITE_MESSAGE", &message);

            let env = RuntimeEnvironment::detect().unwrap();

            assert_eq!(env.ci, "buildkite");
            assert_eq!(env.key, build_id);
            assert_eq!(env.url, Some(url));
            assert_eq!(env.branch, Some(branch));
            assert_eq!(env.commit_sha, Some(commit_sha));
            assert_eq!(env.number, Some(number));
            assert_eq!(env.job_id, Some(job_id));
            assert_eq!(env.message, Some(message));
            assert_eq!(env.version, VERSION);
            assert_eq!(env.collector, format!("rust-{}", COLLECTOR_NAME.to_string()));
        });
    }

    #[test]
    #[serial]
    fn detect_github_actions_environment() {
        let mut rng = rand::thread_rng();

        with_clean_environment(|| {
            let action = "marty".to_string();
            let run_number = rng.gen_range(0..999).to_string();
            let run_attempt = rng.gen_range(0..999).to_string();
            let repo = "buildkite/test-collector-rust".to_string();
            let run_id = Uuid::new_v4().to_string();
            let branch = "marty".to_string();
            let commit_sha = Uuid::new_v4().to_string().replace('-', "");

            env::set_var("GITHUB_ACTION", &action);
            env::set_var("GITHUB_RUN_NUMBER", &run_number);
            env::set_var("GITHUB_RUN_ATTEMPT", &run_attempt);
            env::set_var("GITHUB_REPOSITORY", &repo);
            env::set_var("GITHUB_RUN_ID", &run_id);
            env::set_var("GITHUB_REF", &branch);
            env::set_var("GITHUB_SHA", &commit_sha);

            let env = RuntimeEnvironment::detect().unwrap();

            assert_eq!(env.ci, "github_actions");
            assert_eq!(
                env.key,
                format!("{}-{}-{}", action, run_number, run_attempt)
            );
            assert_eq!(
                env.url,
                Some(format!(
                    "https://github.com/{}/actions/runs/{}",
                    repo, run_id
                ))
            );
            assert_eq!(env.branch, Some(branch));
            assert_eq!(env.commit_sha, Some(commit_sha));
            assert_eq!(env.number, Some(run_number));
            assert_eq!(env.job_id, None);
            assert_eq!(env.message, None);
            assert_eq!(env.version, VERSION);
            assert_eq!(env.collector, format!("rust-{}", COLLECTOR_NAME.to_string()));
        })
    }

    #[test]
    #[serial]
    fn detect_circle_ci_environment() {
        let mut rng = rand::thread_rng();

        with_clean_environment(|| {
            let build_num = (rng.gen_range(0..999) as usize).to_string();
            let workflow_id = (rng.gen_range(0..999) as usize).to_string();
            let commit_sha = Uuid::new_v4().to_string().replace('-', "");
            let url = "https://example.test".to_string();
            let branch = "marty".to_string();

            env::set_var("CIRCLE_BUILD_NUM", &build_num);
            env::set_var("CIRCLE_WORKFLOW_ID", &workflow_id);
            env::set_var("CIRCLE_BUILD_URL", &url);
            env::set_var("CIRCLE_BRANCH", &branch);
            env::set_var("CIRCLE_SHA1", &commit_sha);

            let env = RuntimeEnvironment::detect().unwrap();

            assert_eq!(env.ci, "circleci");
            assert_eq!(env.key, format!("{}-{}", &workflow_id, &build_num));
            assert_eq!(env.url, Some(url));
            assert_eq!(env.branch, Some(branch));
            assert_eq!(env.commit_sha, Some(commit_sha));
            assert_eq!(env.number, Some(build_num));
            assert_eq!(env.job_id, None);
            assert_eq!(env.message, None);
            assert_eq!(env.version, VERSION);
            assert_eq!(env.collector, format!("rust-{}", COLLECTOR_NAME.to_string()));
        });
    }

    #[test]
    #[serial]
    fn detect_generic_environment() {
        with_clean_environment(|| {
            env::set_var("CI", "true");

            let env = RuntimeEnvironment::detect().unwrap();

            assert_eq!(env.ci, "generic");
            assert!(Uuid::parse_str(&env.key).is_ok());

            assert_eq!(env.number, None);
            assert_eq!(env.job_id, None);
            assert_eq!(env.branch, None);
            assert_eq!(env.commit_sha, None);
            assert_eq!(env.message, None);
            assert_eq!(env.url, None);
            assert_eq!(env.version, VERSION);
            assert_eq!(env.collector, format!("rust-{}", COLLECTOR_NAME.to_string()));
        });
    }

    #[test]
    #[serial]
    fn detect_failed() {
        with_clean_environment(|| assert!(RuntimeEnvironment::detect().is_none()))
    }

    fn with_clean_environment<F: FnOnce()>(test: F) {
        let pre_test_env = env::vars().collect::<HashMap<String, String>>();

        let pre_test_ci_keys = pre_test_env
            .keys()
            .filter(|key| is_ci_key(key))
            .collect::<Vec<&String>>();

        for key in &pre_test_ci_keys {
            env::remove_var(key);
        }

        test();

        let post_test_env = env::vars().collect::<HashMap<String, String>>();
        let post_test_ci_keys = post_test_env
            .keys()
            .filter(|key| is_ci_key(key))
            .collect::<Vec<&String>>();

        for key in post_test_ci_keys {
            env::remove_var(key);
        }

        for key in pre_test_ci_keys {
            env::set_var(key, pre_test_env.get(key).unwrap());
        }
    }

    fn is_ci_key(key: &str) -> bool {
        key.starts_with("BUILDKITE")
            || key.starts_with("GITHUB")
            || key.starts_with("CIRCLE")
            || key.starts_with("CI")
    }
}
