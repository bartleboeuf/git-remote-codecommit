# git-remote-codecommit

[![Release](https://github.com/bartleboeuf/git-remote-codecommit/actions/workflows/release.yml/badge.svg)](https://github.com/bartleboeuf/git-remote-codecommit/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-Apache-blue.svg)](LICENSE)

A Rust implementation of the AWS CodeCommit remote helper for Git. This project aims to provide a maintained alternative to the [official Python implementation](https://github.com/aws/git-remote-codecommit).

The adapted Rust implementation was done by Bart LEBOEUF on July 13, 2025.

## Overview

This Git remote helper enables pushing and pulling from AWS CodeCommit repositories using Git over HTTPS. It handles AWS authentication and credential management automatically.

## Features

- Support for AWS CodeCommit repositories over HTTPS
- Automatic AWS credentials management from environment or credentials file
- Support for AWS profiles and regions
- Cross-platform support (Windows, Linux, and macOS)
- Multi-architecture support (x86_64 and arm64)
- Native performance with Rust implementation
- Self-contained dynamically linked binary
- Context-aware error messages for common authentication issues

## Prerequisites

- **Git** 2.0 or newer
- **AWS credentials** configured (see [Configuration](#configuration) section)
- **Rust 1.93.0+** (for local builds; install from [rustup.rs](https://rustup.rs))
- **Docker** (optional, for cross-platform builds)

**Supported platforms:**
- Linux (x86_64, aarch64)
- macOS (Intel, Apple Silicon)
- Windows 10/11

## Installation

### Using Cargo (Recommended)

If you have Rust installed:

```bash
cargo install --path .
```

### Using Docker (Cross-Platform Builds)

```bash
# Clone the repository
git clone https://github.com/bartleboeuf/git-remote-codecommit
cd git-remote-codecommit

# Build using Docker (automatically detects OS and architecture)
chmod +x build.sh
./build.sh

# Copy the binary to your PATH
# For Linux x86_64:
sudo cp target/x86_64-unknown-linux-gnu/release/git-remote-codecommit /usr/local/bin/
# For Linux arm64:
sudo cp target/aarch64-unknown-linux-gnu/release/git-remote-codecommit /usr/local/bin/
# For macOS Intel:
sudo cp target/x86_64-apple-darwin/release/git-remote-codecommit /usr/local/bin/
# For macOS Apple Silicon:
sudo cp target/aarch64-apple-darwin/release/git-remote-codecommit /usr/local/bin/
```

## Quick Start

```bash
# 1. Install
cargo install --path .

# 2. Configure AWS credentials (if not already done)
aws configure

# 3. Clone a CodeCommit repository
git clone codecommit://my-repo

# 4. Start working!
cd my-repo
```

## Usage

### Clone a Repository

```bash
# Using default AWS profile (falls back to us-east-1 region)
git clone codecommit://repository-name

# Using a specific AWS profile
git clone codecommit://profile@repository-name

# Using a specific AWS region
git clone codecommit::us-west-2://repository-name

# Using both region and profile
git clone codecommit::us-west-2://profile@repository-name
```

### Add a Remote

```bash
git remote add origin codecommit://repository-name
```

## Configuration

The helper uses AWS credentials in this order:

1. **Environment variables**: `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`
2. **AWS credentials file**: `~/.aws/credentials`
3. **AWS configuration file**: `~/.aws/config`
4. **AWS SSO profiles**: Via `aws sso login --profile <name>`

### Setting Up AWS Credentials

**Using IAM Access Keys:**
```bash
aws configure
# Enter your AWS Access Key ID and Secret Access Key
```

**Using AWS SSO:**
```bash
aws configure sso
aws sso login --profile your-profile
git clone codecommit://your-profile@repository-name
```

**Using Environment Variables:**
```bash
export AWS_ACCESS_KEY_ID=your-access-key
export AWS_SECRET_ACCESS_KEY=your-secret-key
git clone codecommit://repository-name
```

## Examples

```bash
# Clone using default profile in us-east-1
git clone codecommit://my-repo

# Clone using specific region
git clone codecommit::us-east-1://my-repo

# Clone using 'development' profile
git clone codecommit://development@my-repo

# Clone using specific region and profile
git clone codecommit::eu-west-1://staging@my-repo

# Add remote using specific region and profile
git remote add origin codecommit::us-west-2://production@my-repo
```

## Error Handling

The helper provides context-aware error messages to help you troubleshoot:

**Empty repository name:**
```
Invalid repository name in URL: codecommit:///

Repository name cannot be empty.

Examples:
• codecommit://my-repo
• codecommit://profile@my-repo
• codecommit::us-east-1://my-repo
```

**Invalid AWS region:**
```
The following AWS Region is not available for use with AWS CodeCommit: eu-south-5.

Available regions: af-south-1, ap-east-1, ap-northeast-1, ... [31 total]
```

**SSO session expired:**
```
AWS authentication failed: Your session token is invalid or has expired.

This usually happens when:
• Your AWS SSO session has expired
• You haven't logged in with 'aws sso login'
• Your temporary credentials have expired

Try running: aws sso login --profile myprofile
```

**Credentials not found:**
```
AWS credentials not found.

Please configure your AWS credentials using one of:
• aws configure (for access keys)
• aws sso login --profile myprofile (for SSO)
• Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY environment variables
```

## Development

### Building from Source

**Prerequisites:**
- Rust 1.93.0+ (MSRV)

**Build with quality checks (required for releases):**
```bash
# Clone the repository
git clone https://github.com/bartleboeuf/git-remote-codecommit
cd git-remote-codecommit

# Check code quality (clippy linter)
cargo clippy --all-targets --all-features -- -D warnings

# Build with optimizations and crypto acceleration
cargo build --release --features fast-crypto

# Format check
cargo fmt --check

# Run tests
cargo test
```

**Development build (no optimizations):**
```bash
cargo build
```

## Differences from Official Implementation

- Written in Rust instead of Python
- Self-contained binary (no external dependencies or runtime)
- No Python runtime requirement
- Improved error handling with context-aware messages
- Native performance and smaller binary size
- Cross-platform binary releases

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## License

This project is licensed under the Apache License 2.0 - see the LICENSE file for details.

## Acknowledgments

- Original [git-remote-codecommit](https://github.com/aws/git-remote-codecommit) project by AWS
- AWS SDK for Rust team

## Security

For security concerns, please open an issue or contact the maintainers directly.

## Troubleshooting

### Common Issues

**1. Permission Denied**
```bash
chmod +x /usr/local/bin/git-remote-codecommit
```

**2. AWS Credentials Not Found**
```bash
# Check if credentials are configured
aws configure list

# Configure credentials if needed
aws configure

# Or set environment variables
export AWS_ACCESS_KEY_ID=your-key
export AWS_SECRET_ACCESS_KEY=your-secret
```

**3. Git Not Found in PATH**
```bash
# Ensure git is installed
git --version

# Verify git location
which git
```

**4. SSO Session Expired**
```bash
# Re-authenticate with SSO
aws sso login --profile your-profile
```

**5. Repository Not Found**
- Verify the repository exists in AWS CodeCommit
- Check you have access to the AWS account
- Verify you're using the correct repository name and region

## Support

For bugs and feature requests:
1. Check [existing issues](https://github.com/bartleboeuf/git-remote-codecommit/issues)
2. If not found, create a new issue with:
   - Your OS and version
   - Git version (`git --version`)
   - Rust version (`rustc --version`)
   - Steps to reproduce
   - Expected vs actual behavior
   - Full error message output
