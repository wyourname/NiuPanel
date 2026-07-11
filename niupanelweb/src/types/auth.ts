export type UserPreferences = Record<string, unknown>;

export interface UserInfo {
  id?: number;
  username?: string;
  email?: string | null;
  email_verified?: boolean;
  role?: string;
  permissions?: string[];
  last_login_at?: number | null;
  last_login_ip?: string | null;
  preferences?: UserPreferences;
}

export interface SetupStatus {
  initialized: boolean;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export type LoginResponse = UserInfo | { ticket: string };

export interface VerifyLogin2FARequest {
  ticket: string;
  code: string;
}

export interface RegisterRequest {
  username: string;
  password: string;
  email?: string | null;
  mail_host?: string | null;
  mail_username?: string | null;
  mail_password?: string | null;
}

export interface ResetPasswordRequest {
  token: string;
  new_password: string;
}

export interface ChangePasswordRequest {
  old_password: string;
  new_password: string;
}

export interface EmailChangeRequest {
  new_email: string;
  password_confirm: string;
}
