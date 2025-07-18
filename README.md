# git-remote-codecommit

[![Crates.io](https://img.shields.io/crates/v/git-remote-codecommit.svg)](https://crates.io/crates/git-remote-codecommit)
[![Build Status](https://github.com/bartleboeuf/git-remote-codecommit/workflows/CI/badge.svg)](https://github.com/bartleboeuf/git-remote-codecommit/actions)

A Rust implementation of the AWS CodeCommit remote helper for Git. This project aims to provide a maintained alternative to the [official Python implementation](https://github.com/aws/git-remote-codecommit).

The adapted Rust implementation was done by Bart LEBOEUF on July 13, 2025.

## Overview

This Git remote helper enables pushing and pulling from AWS CodeCommit repositories using Git over HTTPS. It handles AWS authentication and credential management automatically.

## Features

- Support for AWS CodeCommit repositories
- Automatic AWS credentials management
- Support for AWS profiles
- Cross-platform compatibility
- Statically linked binary (no runtime dependencies)
- Smaller footprint compared to the Python implementation

## Prerequisites

- Git (2.0 or newer)
- AWS credentials configured (`~/.aws/credentials` or environment variables)
- Docker (for building without Rust installation)
- Linux, macOS, or Windows

## Installation

### Using Docker (recommended)

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

### Manual Installation

If you have Rust installed:

```bash
cargo install --path .
```

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/bartleboeuf/git-remote-codecommit
cd git-remote-codecommit

# Build with optimizations
cargo build --release

# Run tests
cargo test
```

## Usage

### Clone a repository

```bash
# Using default AWS profile
git clone codecommit://repository-name

# Using a specific AWS profile
git clone codecommit://profile@repository-name

# Using a specific AWS region
git clone codecommit::region://profile@repository-name
```

### Add a remote

```bash
git remote add origin codecommit://repository-name
```

## Configuration

The helper uses your AWS credentials from:
1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. AWS credentials file (`~/.aws/credentials`)
3. AWS configuration file (`~/.aws/config`)

## Examples

```bash
# Clone using default profile in us-east-1
git clone codecommit::us-east-1://my-repo

# Clone using 'development' profile
git clone codecommit://development@my-repo

# Add remote using specific region and profile
git remote add origin codecommit::eu-west-1://staging@my-repo
```

## Differences from Official Implementation

- Written in Rust instead of Python
- Smaller binary size
- No Python runtime dependency
- Improved error handling
- Static linking (more portable)

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

1. **Permission Denied**
   ```bash
   chmod +x /usr/local/bin/git-remote-codecommit
   ```

2. **AWS Credentials Not Found**
   ```bash
   export AWS_PROFILE=your-profile
   # or
   export AWS_ACCESS_KEY_ID=your-key
   export AWS_SECRET_ACCESS_KEY=your-secret
   ```

## Support

For bugs and feature requests, please:
1. Search existing issues
2. If not found, create a new issue with:
   - Your OS and version
   - Git version (`git --version`)
   - Steps to reproduce
   - Expected vs actual behavior
