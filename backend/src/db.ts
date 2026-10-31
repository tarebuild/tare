import { PrismaClient } from '@prisma/client'
import { createDatabase } from './engines/server/db.js'

export function createDb(url: string) {
  return createDatabase(url, (adapter) => new PrismaClient({ adapter }))
}

export type Database = ReturnType<typeof createDb>
