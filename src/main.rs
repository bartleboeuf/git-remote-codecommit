#![forbid(unsafe_code)]

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_credential_types::provider::ProvideCredentials;
use aws_types::region::Region;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::process::{Command, exit};
use url::Url;

type HmacSha256 = Hmac<Sha256>;

fn error_exit(msg: &str) {
    eprintln!("{msg}");
    exit(1);
}

const KNOWN_REGIONS: &[&str] = &[
    "af-south-1", "ap-east-1", "ap-northeast-1", "ap-northeast-2", "ap-northeast-3",
    "ap-south-1", "ap-southeast-1", "ap-southeast-2", "ap-southeast-3", "ca-central-1",
    "cn-north-1", "cn-northwest-1", "eu-central-1", "eu-north-1", "eu-south-1",
    "eu-west-1", "eu-west-2", "eu-west-3", "il-central-1", "me-central-1",
    "me-south-1", "sa-east-1", "us-east-1", "us-east-2", "us-gov-east-1",
    "us-gov-west-1", "us-west-1", "us-west-2"
];

#[inline]
fn is_region_available(region: &str) -> bool {
    KNOWN_REGIONS.contains(&region)
}

#[inline]
fn website_domain_mapping(region: &str) -> &'static str {
    match region {
        "cn-north-1" | "cn-northwest-1" => "amazonaws.com.cn",
        _ => "amazonaws.com",
    }
}

fn hash_sha256(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    // Pre-allocate with exact capacity (64 hex chars)
    let mut result = String::with_capacity(64);
    result.push_str(&format!("{:x}", hasher.finalize()));
    result
}

fn hmac_sha256(key: &[u8], input: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(input.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn generate_signature(hostname: &str, path: &str, region: &str, creds: &Credentials) -> String {
    use chrono::Utc;
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S").to_string();

    let canonical_request = format!("GIT\n{path}\n\nhost:{hostname}\n\nhost\n");
    let credential_scope = format!("{}/{}/codecommit/aws4_request", &timestamp[..8], region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        timestamp,
        credential_scope,
        hash_sha256(&canonical_request)
    );

    let date_key = hmac_sha256(
        format!("AWS4{}", creds.secret_access_key()).as_bytes(),
        &timestamp[..8]
    );
    let date_region_key = hmac_sha256(&date_key, region);
    let date_region_service_key = hmac_sha256(&date_region_key, "codecommit");
    let signing_key = hmac_sha256(&date_region_service_key, "aws4_request");
    let signature = hex::encode(hmac_sha256(&signing_key, &string_to_sign));

    format!("{timestamp}Z{signature}")
}

fn build_git_url(repository: &str, region: &str, creds: &Credentials) -> String {
    let hostname = env::var("CODE_COMMIT_ENDPOINT").unwrap_or_else(|_| 
        format!(
            "git-codecommit.{}.{}",
            region,
            website_domain_mapping(region)
        ));

    let path = format!("/v1/repos/{repository}");
    let username_raw = match creds.session_token() {
        Some(token) => format!("{}%{}", creds.access_key_id(), token),
        None => creds.access_key_id().to_string(),
    };

    let username = urlencoding::encode(&username_raw);
    let signature = generate_signature(&hostname, &path, region, creds);

    format!("https://{username}:{signature}@{hostname}{path}")
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        error_exit("Too few arguments. This hook requires the git command and remote.");
    }
    if args.len() > 3 {
        error_exit(&format!(
            "Too many arguments. Hook only accepts the git command and remote, but argv was: '{}'",
            args.join("', '")
        ));
    }

    let git_cmd = &args[1];
    let remote_url = &args[2];

    // Parse remote_url
    let url = Url::parse(remote_url).unwrap_or_else(|_| {
        error_exit(&format!("Malformed URL: {remote_url}. Must be codecommit://<repository> or codecommit::<region>://<repository>"));
        unreachable!()
    });

    let mut profile = "default".to_string();
    let mut repository = url.host_str().unwrap_or("").to_string();
    let region = url.scheme().to_string();

    // Parse profile from URL - check User info first, then fallback to Host parsing
    if let Some(user) = url.username().split('@').next() {
        if !user.is_empty() {
            profile = user.to_string();
        }
    } else if repository.contains('@') {
        let parts: Vec<&str> = repository.splitn(2, '@').collect();
        if parts.len() == 2 {
            profile = parts[0].to_string();
            repository = parts[1].to_string();
        }
    }

    if !is_region_available(&region) {
        error_exit(&format!("The following AWS Region is not available for use with AWS CodeCommit: {region}."));
    }

    // Load AWS config
    let mut config_builder =
        aws_config::defaults(BehaviorVersion::latest()).region(Region::new(region.to_string()));

    if profile != "default" {
        config_builder = config_builder.profile_name(&profile);
    }

    let config = config_builder.load().await;

    // Get credentials from the config
    let credential_result = config
        .credentials_provider()
        .expect("No credentials provider")
        .provide_credentials()
        .await;

    let credentials = match credential_result {
        Ok(creds) => creds,
        Err(e) => {
            // Check for specific error types and provide human-readable messages
            let error_msg = if let Some(source) = e.source() {
                let error_str = source.to_string();
                if error_str.contains("Session token not found or invalid") {
                    format!(
                        "AWS authentication failed: Your session token is invalid or has expired.\n\
                        \n\
                        This usually happens when:\n\
                        • Your AWS SSO session has expired\n\
                        • You haven't logged in with 'aws sso login'\n\
                        • Your temporary credentials have expired\n\
                        \n\
                        Try running: aws sso login --profile {profile}")
                } else if error_str.contains("UnauthorizedException") {
                    format!(
                        "AWS authentication failed: You don't have permission to access CodeCommit.\n\
                        \n\
                        Please check:\n\
                        • Your AWS credentials are configured correctly\n\
                        • Your user/role has CodeCommit permissions\n\
                        • You're using the correct AWS profile ({profile})")
                } else if error_str.contains("NoCredentialsError") || error_str.contains("CredentialsNotLoaded") {
                    format!(
                        "AWS credentials not found.\n\
                        \n\
                        Please configure your AWS credentials using one of:\n\
                        • aws configure (for access keys)\n\
                        • aws sso login --profile {profile} (for SSO)\n\
                        • Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY environment variables")
                } else {
                    format!(
                        "Failed to load AWS credentials for profile '{profile}'.\n\
                        \n\
                        Error details: {error_str}\n\
                        \n\
                        Try:\n\
                        • Check your AWS configuration: aws configure list --profile {profile}\n\
                        • Verify your profile exists: aws configure list-profiles\n\
                        • Re-authenticate if using SSO: aws sso login --profile {profile}")
                }
            } else {
                format!(
                    "Failed to load AWS credentials for profile '{profile}'.\n\
                    \n\
                    Please ensure your AWS credentials are properly configured.")
            };

            error_exit(&error_msg);
            unreachable!()
        }
    };


    let authenticated_url = build_git_url(&repository, &region, &credentials);

    // Execute git remote-http with the authenticated URL
    let status = Command::new("git")
        .arg("remote-http")
        .arg(git_cmd)
        .arg(&authenticated_url)
        .status()
        .expect("Failed to execute git remote-http");

    if !status.success() {
        exit(status.code().unwrap_or(1));
    }

    Ok(())
}