# Product Guidelines: Shadowbox

## User Experience (UX) Principles
- **Rich & Interactive CLI**: The interface should provide clear, visual feedback using colors, icons, and progress bars. It should be engaging while remaining highly functional for power users.
- **Background Key Management**: Leverage native system keychains (e.g., macOS Keychain, Windows Credential Manager) to securely store and retrieve encryption keys without constant user prompting.
- **Seamless Flow**: Minimize friction during project switches; the tool should "just work" in the background based on the current directory or project context.

## Design Standards
- **Privacy-Preserving Labels**: When storing data in third-party clouds, use obscured or hashed names for project buckets/folders to prevent metadata leakage, while maintaining internal mapping for the user.
- **Consistency**: Use standardized terminology for sync states (e.g., `Synchronized`, `Pending`, `Conflict`, `Error`).

## Operational Guidelines
- **Smart Conflict Resolution**: In the event of a sync conflict, the tool should attempt to resolve it automatically using logic (e.g., most recent timestamp, file size, or content hash) before falling back to a manual prompt.
- **Resilient Connectivity**: Gracefully handle network interruptions; use exponential backoff for retries and provide clear status updates on connectivity issues.
- **Data Integrity**: Always perform checksum verification before and after synchronization to ensure no data corruption occurs.
