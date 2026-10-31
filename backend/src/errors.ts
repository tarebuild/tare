import { errors, type ErrorCode } from '@tare/contract'
import { failer, type Fail } from './engines/server/errors.js'
export const fail: Fail<ErrorCode> = failer(errors.status)
