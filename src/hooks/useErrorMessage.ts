import { i18n } from '@/locales';

const translate = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});

const normalizeErrorDetail = (error: unknown) => {
  if (typeof error === 'string') {
    return error;
  }

  if (error instanceof Error) {
    return error.message || error.name;
  }

  if (typeof error === 'object' && error !== null) {
    try {
      const serialized = JSON.stringify(error);

      if (serialized && serialized !== '{}') {
        return serialized;
      }
    } catch {
      return String(error);
    }
  }

  return String(error);
};

const MAX_ERROR_MESSAGE_CHARS = 280;

const truncateErrorMessage = (value: string) => {
  const trimmed = value.trim();

  if (trimmed.length <= MAX_ERROR_MESSAGE_CHARS) {
    return trimmed;
  }

  return `${trimmed.slice(0, MAX_ERROR_MESSAGE_CHARS)}...`;
};

export interface CodedError {
  code: string;
  message: string;
  params?: Record<string, string>;
}

/** Rust AppError 序列化形状：{ code, message, params }；旧命令仍是纯字符串 */
export const isCodedError = (error: unknown): error is CodedError =>
  typeof error === 'object' &&
  error !== null &&
  typeof (error as CodedError).code === 'string' &&
  typeof (error as CodedError).message === 'string';

const translateCodedError = (error: CodedError): string => {
  const key = `errors.${error.code}`;
  const translated = translate(key, error.params ?? {});

  // 缺失词条时 vue-i18n 回退中文；连中文词条也没有则回原始 message
  return translated === key ? error.message : translated;
};

export const formatErrorMessage = (description: string, error: unknown) => {
  if (isCodedError(error)) {
    return translateCodedError(error);
  }

  const detail = truncateErrorMessage(normalizeErrorDetail(error));

  if (!detail) {
    return description;
  }

  return `${description}${translate('common.colon')}${detail}`;
};
