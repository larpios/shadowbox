# Tech Stack: Shadowbox

## Core Technologies
- **Rust (Primary)**: Use Rust for the performance-critical core sync logic, encryption/decryption, and high-safety CLI components.
- **Go (Networking)**: Leverage Go's concurrency model for handling multi-cloud parallel uploads and complex networking tasks.
- **Python (Integration/Scripting)**: Use Python for rapid prototyping of integration scripts, user-facing configuration tools, and non-performance critical tasks.

## Security & Encryption
- **libsodium (NaCl)**: Utilize libsodium for high-level, easy-to-use modern cryptography (Signatures, Encrypted Boxes).
- **AES-GCM**: Use AES-GCM for standard object-level symmetric encryption of large file payloads.
- **Noise Protocol**: Implement secure channel communication for peer-to-peer sync and metadata exchange.

## Storage & Cloud
- **AWS SDK for S3**: Primary cloud backend for object storage, supporting versioning and lifecycle policies.
- **Google Cloud SDK**: Secondary backend for users preferring the Google ecosystem.
- **Dropbox API**: Third-party file-based sync for documents and shared folders.

## State Management
- **SQLite**: Local relational database for tracking file hashes, sync states, project-to-bucket mappings, and local/remote configuration.
