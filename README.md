# Public Ollama Server Finder

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Built_With-Rust-orange.svg)](https://www.rust-lang.org)
[![Release](https://img.shields.io/github/v/release/zonay/public-ollama-finder)](https://github.com/zonay/public-ollama-finder/releases)

## Introduction
This project provides a comprehensive network scanning tool designed to discover and enumerate accessible Ollama servers across specified network ranges. Built with Rust for optimal performance and safety, it leverages the language's concurrent programming capabilities to efficiently scan large IP ranges while maintaining minimal resource usage.

### Key Capabilities
- **Network Discovery**: Efficiently scans IPv4 ranges using CIDR notation or IP ranges
- **Model Detection**: Identifies and lists available LLMs, their sizes, and configurations
- **Detailed Reporting**: Generates structured CSV outputs for further analysis
- **Cross-Platform**: Supports Windows, Linux, and macOS with native executables

### Technical Highlights
- **Built with Rust**: Ensures memory safety and thread safety with zero-cost abstractions
- **Concurrent Scanning**: Utilizes Rust's async/await for efficient parallel processing
- **Resource Efficient**: Optimized memory usage and connection pooling
- **Cross-Platform**: Native compilation for all major operating systems

## Installation & Usage

1. Download the appropriate executable for your system from the [Releases page](https://github.com/zonay/public-ollama-finder/releases):
   - Windows: `public-ollama-finder-windows.exe`
   - Linux: `public-ollama-finder-linux`
   - macOS: `public-ollama-finder-macos`

2. Create a file named `ip-ranges.txt` in the same directory as the executable.

   ### IP Range Examples:
   Your ip-ranges.txt file can contain any of these formats:
   ```
   # CIDR notation
   192.168.1.0/24
   10.0.0.0/8

   # IP ranges
   192.168.1.1-192.168.1.255
   10.0.0.1-10.0.0.100

   # Single IPs
   192.168.1.42
   10.0.0.5

   # Mixed formats are supported
   172.16.0.0/16
   192.168.1.1-192.168.1.10
   10.0.0.1
   ```

   ### Running the Scanner

   **Windows**:
   ```cmd
   .\public-ollama-finder-windows.exe
   ```

   **Linux**:
   ```bash
   chmod +x public-ollama-finder-linux
   ./public-ollama-finder-linux
   ```

   **macOS**:
   ```bash
   chmod +x public-ollama-finder-macos
   ./public-ollama-finder-macos
   ```

3. The scanner will generate two CSV files:
   - `ollama_endpoints.csv`: Lists discovered endpoints.
   - `llm_models.csv`: Lists discovered language models per endpoint.

## Sample Output

<details>
<summary>Click to view sample console output</summary>

```
╭─ Public Ollama Finder
├─ Repository: github.com/zonay/public-ollama-finder
├─ Targets: 3 IP ranges (65534 total IPs)
├─ Port: 11434 /api/tags
╰─ Controls: [p]ause [r]esume [q]uit | Ctrl+C to stop

⠹ [██████████████████░░░░░░░░░░░░░░░░] 45% • 29876/65534 IPs

╭─ Found Ollama Server
├─ API Endpoint: http://192.168.1.100:11434/api/tags
├─ Server URL: http://192.168.1.100:11434
├─ Available Models:
   ├─ 1. llama2 (7.03 GB)
   ├─ 2. mistral (7.09 GB)
   ╰─ 3. codellama (7.16 GB)
```
</details>

<details>
<summary>Click to view sample CSV output</summary>

```csv
# ollama_endpoints.csv
IP:Port,Tags URL,Status Code,Location
http://192.168.1.100:11434,http://192.168.1.100:11434/api/tags,200,Local

# llm_models.csv
IP:Port,Model Name,Model,Modified At,Size,Parent Model,Format,Family,Parameter Size,Quantization Level
http://192.168.1.100:11434,llama2,llama2:7b,2024-01-20,7.03,llama2,gguf,llama,7B,Q4_K_M
```
</details>

## Development

### Prerequisites
- Rust 1.70.0 or later
- Cargo (included with Rust)
- Git

### Quick Start
```bash
git clone https://github.com/zonay/public-ollama-finder.git
cd public-ollama-finder
cargo run
```

### Release Process
```bash
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

## Contributing

We welcome contributions! Here's how you can help:
1. Fork this repository
2. Create a feature branch
3. Submit a pull request

## License
This project is licensed under the MIT License.

## Important Legal Notice and Disclaimer

**This tool is for educational and authorized security testing purposes only.**

### Critical Warning:
**Scanning servers without explicit permission** may result in serious consequences:
- Legal actions and prosecution
- Network-wide IP bans
- Defensive countermeasures

### LEGAL CONFIRMATION:
By using this tool, you explicitly confirm:
1. **You have authorization for all target networks**
2. **You accept full responsibility for your actions**
3. **You understand all legal implications**

The developer of this tool assume no liability for misuse or unauthorized scanning activities.
