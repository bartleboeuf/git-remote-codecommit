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

fn error_exit(msg: &str) -> ! {
    eprintln!("{msg}");
    exit(1);
}

/// Supported AWS CodeCommit regions that `_is_region_available()` can validate.
/// This list must remain sorted because `binary_search` relies on ordering.
const KNOWN_REGIONS: &[&str] = &[
    "af-south-1",
    "ap-east-1",
    "ap-northeast-1",
    "ap-northeast-2",
    "ap-northeast-3",
    "ap-south-1",
    "ap-south-2",
    "ap-southeast-1",
    "ap-southeast-2",
    "ap-southeast-3",
    "ca-central-1",
    "cn-north-1",
    "cn-northwest-1",
    "eu-central-1",
    "eu-north-1",
    "eu-south-1",
    "eu-west-1",
    "eu-west-2",
    "eu-west-3",
    "eusc-de-east-1",
    "il-central-1",
    "me-central-1",
    "me-south-1",
    "sa-east-1",
    "us-east-1",
    "us-east-2",
    "us-gov-east-1",
    "us-gov-west-1",
    "us-west-1",
    "us-west-2",
];

#[inline]
/// Returns whether the helper can operate in `region`.
/// The sorted slice enables a fast `binary_search` instead of a full linear scan.
fn is_region_available(region: &str) -> bool {
    KNOWN_REGIONS.binary_search(&region).is_ok()
}

#[inline]
/// Chooses the correct DNS suffix for the given region.
/// Mainland China uses `.amazonaws.com.cn` while the European sovereign cloud uses `.amazonaws.eu`.
fn website_domain_mapping(region: &str) -> &'static str {
    match region {
        "cn-north-1" | "cn-northwest-1" => "amazonaws.com.cn",
        _ if region.starts_with("eusc-") => "amazonaws.eu",
        _ => "amazonaws.com",
    }
}

/// Returns the SHA256 digest of `input`.
fn hash_sha256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let digest = hasher.finalize();
    let mut out = [0_u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Produces an HMAC-SHA256 using `key` and `input`, keeping the result on the stack.
fn hmac_sha256(key: &[u8], input: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(input);
    let digest = mac.finalize().into_bytes();
    let mut out = [0_u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Builds the signed `username:password` fragment needed by `git remote-http`.
/// Implements AWS Signature Version 4 (AWS SigV4) signing process:
/// 1. Create canonical request with GIT verb
/// 2. Hash canonical request (SHA256)
/// 3. Create string to sign with datestamp, region, service scope
/// 4. Derive signing key using HMAC chain: AWS4 + secret → date → region → service → request
/// 5. Sign the string to produce final signature
fn generate_signature(hostname: &str, path: &str, region: &str, creds: &Credentials) -> String {
    use chrono::Utc;
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S").to_string();
    let datestamp = &timestamp[..8];

    // Step 1: Build canonical request (GIT verb for CodeCommit)
    let mut canonical_request = String::with_capacity(path.len() + hostname.len() + 16);
    canonical_request.push_str("GIT\n"); // HTTP verb for AWS CodeCommit signing
    canonical_request.push_str(path);
    canonical_request.push_str("\n\nhost:");
    canonical_request.push_str(hostname);
    canonical_request.push_str("\n\nhost\n");

    // Step 2: Hash canonical request
    let canonical_request_hash = hash_sha256(canonical_request.as_bytes());

    let mut canonical_request_hash_hex = [0_u8; 64];
    hex::encode_to_slice(canonical_request_hash, &mut canonical_request_hash_hex)
        .expect("sha256 digest must encode to 64 hex chars");
    let canonical_request_hash_hex = std::str::from_utf8(&canonical_request_hash_hex)
        .expect("hex-encoded digest is valid UTF-8");

    // Step 3: Build credential scope for CodeCommit service
    let credential_scope = format!("{}/{}/codecommit/aws4_request", datestamp, region);

    // Step 4: Build string to sign (what we'll sign with the derived key)
    let mut string_to_sign = String::with_capacity(
        32 + timestamp.len() + credential_scope.len() + canonical_request_hash_hex.len(),
    );
    string_to_sign.push_str("AWS4-HMAC-SHA256\n");
    string_to_sign.push_str(&timestamp);
    string_to_sign.push('\n');
    string_to_sign.push_str(&credential_scope);
    string_to_sign.push('\n');
    string_to_sign.push_str(canonical_request_hash_hex);

    // Step 5: Derive signing key using HMAC chain
    // This prevents key reuse across services, regions, and dates
    let secret = creds.secret_access_key().as_bytes();
    let mut aws4_secret = Vec::with_capacity(4 + secret.len());
    aws4_secret.extend_from_slice(b"AWS4"); // AWS4 prefix
    aws4_secret.extend_from_slice(secret);

    let date_key = hmac_sha256(&aws4_secret, datestamp.as_bytes()); // HMAC with date
    let date_region_key = hmac_sha256(&date_key, region.as_bytes()); // HMAC with region
    let date_region_service_key = hmac_sha256(&date_region_key, b"codecommit"); // HMAC with service
    let signing_key = hmac_sha256(&date_region_service_key, b"aws4_request"); // HMAC with request type

    // Step 6: Sign the string to sign with derived key
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

    format!("{timestamp}Z{signature}")
}

/// Constructs the HTTPS URL that contains credentials recognized by `git remote-http`.
/// The placeholder `CODE_COMMIT_ENDPOINT` env var allows overriding the AWS endpoint for testing.
fn build_git_url(repository: &str, region: &str, creds: &Credentials) -> String {
    let hostname = env::var("CODE_COMMIT_ENDPOINT").unwrap_or_else(|_| {
        format!(
            "git-codecommit.{}.{}",
            region,
            website_domain_mapping(region)
        )
    });

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
/// Entrypoint for the git credential helper.
/// - Parses the `codecommit://` URL
/// - Resolves AWS credentials
/// - Builds the signed remote URL
/// - Defers the actual git operation to `git remote-http`
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

    // Parse remote_url (format: codecommit://[profile@]repository or codecommit::region://[profile@]repository)
    let url = Url::parse(remote_url).unwrap_or_else(|_| {
        error_exit(&format!(
            "Malformed URL: {remote_url}\n\
            \n\
            Expected format:\n\
            • codecommit://repository\n\
            • codecommit://profile@repository\n\
            • codecommit::region://repository\n\
            • codecommit::region://profile@repository\n\
            \n\
            Example: codecommit::us-east-1://myprofile@my-repo"
        ))
    });

    let mut profile = "default".to_string();
    let repository = url
        .host_str()
        .filter(|h| !h.is_empty())
        .map(|h| h.to_string())
        .unwrap_or_else(|| {
            error_exit(&format!(
                "Invalid repository name in URL: {remote_url}\n\
                \n\
                Repository name cannot be empty.\n\
                \n\
                Examples:\n\
                • codecommit://my-repo\n\
                • codecommit://profile@my-repo\n\
                • codecommit::us-east-1://my-repo"
            ))
        });
    let scheme = url.scheme().to_string();

    // Parse profile from URL user info (format: profile@repository)
    let user = url.username();
    if !user.is_empty() {
        profile = user.to_string();
    }

    // Determine region: if scheme is a valid region, use it; otherwise use default
    let region = if is_region_available(&scheme) {
        scheme
    } else if scheme == "codecommit" {
        // No explicit region specified; use fallback.
        // AWS SDK's default resolution is checked later and can override this if configured.
        "us-east-1".to_string()
    } else {
        error_exit(&format!(
            "The following AWS Region is not available for use with AWS CodeCommit: {scheme}.\n\
            \n\
            Available regions: af-south-1, ap-east-1, ap-northeast-1, ap-northeast-2, ap-northeast-3, \
            ap-south-1, ap-south-2, ap-southeast-1, ap-southeast-2, ap-southeast-3, ca-central-1, \
            cn-north-1, cn-northwest-1, eu-central-1, eu-north-1, eu-south-1, eu-west-1, eu-west-2, \
            eu-west-3, eusc-de-east-1, il-central-1, me-central-1, me-south-1, sa-east-1, us-east-1, \
            us-east-2, us-gov-east-1, us-gov-west-1, us-west-1, us-west-2"
        ))
    };

    // Load AWS config
    let mut config_builder =
        aws_config::defaults(BehaviorVersion::latest()).region(Region::new(region.to_string()));

    if profile != "default" {
        config_builder = config_builder.profile_name(&profile);
    }

    let config = config_builder.load().await;

    // Get credentials from the config
    let credentials_provider = config.credentials_provider().unwrap_or_else(|| {
        error_exit(&format!(
            "AWS credentials provider not available.\n\
            \n\
            Please ensure your AWS credentials are properly configured using one of:\n\
            • aws configure (for access keys)\n\
            • aws sso login --profile {} (for SSO)\n\
            • Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY environment variables",
            profile
        ))
    });

    let credential_result = credentials_provider.provide_credentials().await;

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
                        Try running: aws sso login --profile {}",
                        profile
                    )
                } else if error_str.contains("UnauthorizedException") {
                    format!(
                        "AWS authentication failed: You don't have permission to access CodeCommit.\n\
                        \n\
                        Please check:\n\
                        • Your AWS credentials are configured correctly\n\
                        • Your user/role has CodeCommit permissions\n\
                        • You're using the correct AWS profile ({})",
                        profile
                    )
                } else if error_str.contains("NoCredentialsError")
                    || error_str.contains("CredentialsNotLoaded")
                {
                    format!(
                        "AWS credentials not found.\n\
                        \n\
                        Please configure your AWS credentials using one of:\n\
                        • aws configure (for access keys)\n\
                        • aws sso login --profile {} (for SSO)\n\
                        • Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY environment variables",
                        profile
                    )
                } else {
                    format!(
                        "Failed to load AWS credentials for profile '{}'.\n\
                        \n\
                        Error details: {}\n\
                        \n\
                        Try:\n\
                        • Check your AWS configuration: aws configure list --profile {}\n\
                        • Verify your profile exists: aws configure list-profiles\n\
                        • Re-authenticate if using SSO: aws sso login --profile {}",
                        profile, error_str, profile, profile
                    )
                }
            } else {
                format!(
                    "Failed to load AWS credentials for profile '{profile}'.\n\
                    \n\
                    Please ensure your AWS credentials are properly configured."
                )
            };

            error_exit(&error_msg)
        }
    };

    let authenticated_url = build_git_url(&repository, &region, &credentials);

    // Execute git remote-http with the authenticated URL
    let status = Command::new("git")
        .arg("remote-http")
        .arg(git_cmd)
        .arg(&authenticated_url)
        .status()
        .unwrap_or_else(|e| {
            error_exit(&format!(
                "Failed to execute git remote-http: {e}\n\
                \n\
                Please ensure:\n\
                • Git is installed and in your PATH\n\
                • You have permission to execute: git remote-http"
            ))
        });

    if !status.success() {
        exit(status.code().unwrap_or(1));
    }

    Ok(())
}
