use aws_config::BehaviorVersion;
use aws_credential_types::provider::ProvideCredentials;
use aws_credential_types::Credentials;
use aws_types::region::Region;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::env;
use std::process::{Command, exit};
use url::Url;

type HmacSha256 = Hmac<Sha256>;

fn error_exit(msg: &str) {
    eprintln!("{}", msg);
    exit(1);
}

const KNOWN_REGIONS: &[&str] = &[
    "us-east-1", "us-east-2", "us-west-1", "us-west-2",
    "eu-west-1", "eu-west-2", "eu-west-3",
    "eu-central-1", "eu-north-1",
    "ap-southeast-1", "ap-southeast-2",
    "ap-northeast-1", "ap-northeast-2",
    "ap-south-1", "ca-central-1", "sa-east-1",
    "cn-north-1", "cn-northwest-1",
    "us-gov-west-1", "us-gov-east-1",
];

fn is_region_available(region: &str) -> bool {
    KNOWN_REGIONS.contains(&region)
}

fn website_domain_mapping(region: &str) -> &'static str {
    match region {
        "cn-north-1" | "cn-northwest-1" => "amazonaws.com.cn",
        _ => "amazonaws.com",
    }
}

fn hash_sha256(input: &str) -> String {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn hmac_sha256(key: &[u8], input: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(input.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn sign(hostname: &str, path: &str, region: &str, creds: &Credentials) -> String {
    use chrono::Utc;
    use std::sync::OnceLock;
    
    static TIMESTAMP: OnceLock<String> = OnceLock::new();
    let timestamp = TIMESTAMP.get_or_init(|| {
        Utc::now().format("%Y%m%dT%H%M%S").to_string()
    });

    let canonical_request = format!("GIT\n{}\n\nhost:{}\n\nhost\n", path, hostname);
    let algorithm = "AWS4-HMAC-SHA256";
    let credential_scope = format!("{}/{}/codecommit/aws4_request", &timestamp[..8], region);
    let string_to_sign = format!(
        "{}\n{}\n{}\n{}",
        algorithm,
        timestamp,
        credential_scope,
        hash_sha256(&canonical_request)
    );

    let date_key = hmac_sha256(format!("AWS4{}", creds.secret_access_key()).as_bytes(), &timestamp[..8]);
    let date_region_key = hmac_sha256(&date_key, region);
    let date_region_service_key = hmac_sha256(&date_region_key, "codecommit");
    let signing_key = hmac_sha256(&date_region_service_key, "aws4_request");
    let signature = hex::encode(hmac_sha256(&signing_key, &string_to_sign));

    format!("{}Z{}", timestamp, signature)
}

fn git_url(repository: &str, version: &str, region: &str, creds: &Credentials) -> String {
    let hostname = env::var("CODE_COMMIT_ENDPOINT")
        .unwrap_or_else(|_| format!(
            "git-codecommit.{}.{}", 
            region, 
            website_domain_mapping(region)
        ));
    
    let path = format!("/{}/repos/{}", version, repository);
    let username_raw = {
        let mut s = creds.access_key_id().to_owned();
        if let Some(token) = creds.session_token() {
            s.push('%');
            s.push_str(token);
        }
        s
    };
    
    let username = urlencoding::encode(&username_raw);
    let signature = sign(&hostname, &path, region, creds);
    
    format!("https://{}:{}@{}{}", username, signature, hostname, path)
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
        error_exit(&format!(
            "Malformed URL: {}. Must be codecommit://<repository> or codecommit::<region>://<repository>",
            remote_url
        ));
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
        profile = parts[0].to_string();
        repository = parts[1].to_string();
    }

    if !is_region_available(&region) {
        error_exit(&format!(
            "The following AWS Region is not available for use with AWS CodeCommit: {}.",
            region
        ));
    }

    // Load AWS config
    let mut config_builder = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region.to_string()));
        
    if profile != "default" {
        config_builder = config_builder.profile_name(&profile);
    }
    
    let config = config_builder.load().await;
        
    // Get credentials from the config
    let credentials = config.credentials_provider()
        .expect("No credentials provider")
        .provide_credentials()
        .await
        .expect("Failed to get credentials");


    let authenticated_url = git_url(&repository, "v1", &region, &credentials);

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