export interface EncryptPayload {
  code: string;
  versions: string[];
  function_name: string;
  obfuscate?: boolean;
}
