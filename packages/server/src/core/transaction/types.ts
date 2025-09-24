export interface FileSystemSnapshot {
  files: Map<string, string | null>; // File path -> File content (null if file doesn't exist)
}

export interface Transaction {
  id: string;
  checkpoints: Map<string, FileSystemSnapshot>;
}
