-- CreateSchema
CREATE SCHEMA IF NOT EXISTS "public";

-- CreateEnum
CREATE TYPE "PinnedKind" AS ENUM ('report', 'flag_input');

-- CreateTable
CREATE TABLE "records" (
    "address" VARCHAR(44) NOT NULL,
    "program" VARCHAR(44) NOT NULL,
    "programName" TEXT,
    "attester" VARCHAR(44) NOT NULL,
    "sourceRepository" TEXT NOT NULL,
    "sourceCommit" VARCHAR(64) NOT NULL,
    "toolchainVersion" TEXT NOT NULL,
    "stockBuildHash" CHAR(64) NOT NULL,
    "tareBuildHash" CHAR(64) NOT NULL,
    "corpusHash" CHAR(64) NOT NULL,
    "testsCount" INTEGER NOT NULL,
    "generatedCount" INTEGER NOT NULL,
    "mainnetCount" INTEGER NOT NULL,
    "gross" BIGINT NOT NULL,
    "net" BIGINT NOT NULL,
    "reportHash" CHAR(64) NOT NULL,
    "reportLocation" TEXT NOT NULL,
    "slot" BIGINT NOT NULL,
    "signature" VARCHAR(88) NOT NULL,
    "createdAt" TIMESTAMPTZ(3) NOT NULL,

    CONSTRAINT "records_pkey" PRIMARY KEY ("address")
);

-- CreateTable
CREATE TABLE "flags" (
    "address" VARCHAR(44) NOT NULL,
    "record_address" VARCHAR(44) NOT NULL,
    "inputHash" CHAR(64) NOT NULL,
    "inputLocation" TEXT NOT NULL,
    "flagger" VARCHAR(44) NOT NULL,
    "slot" BIGINT NOT NULL,
    "signature" VARCHAR(88) NOT NULL,
    "createdAt" TIMESTAMPTZ(3) NOT NULL,

    CONSTRAINT "flags_pkey" PRIMARY KEY ("address")
);

-- CreateTable
CREATE TABLE "pinned_reports" (
    "hash" CHAR(64) NOT NULL,
    "kind" "PinnedKind" NOT NULL,
    "location" TEXT NOT NULL,
    "bytes" INTEGER NOT NULL,
    "payload" JSONB,
    "createdAt" TIMESTAMPTZ(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "pinned_reports_pkey" PRIMARY KEY ("hash")
);

-- CreateTable
CREATE TABLE "weighings" (
    "program" VARCHAR(44) NOT NULL,
    "sampledAt" TIMESTAMPTZ(3) NOT NULL,
    "payload" JSONB NOT NULL,

    CONSTRAINT "weighings_pkey" PRIMARY KEY ("program")
);

-- CreateTable
CREATE TABLE "indexed_transactions" (
    "signature" VARCHAR(88) NOT NULL,
    "slot" BIGINT NOT NULL,
    "transaction_index" INTEGER NOT NULL DEFAULT 0,
    "block_time" TIMESTAMPTZ(6),
    "failed" BOOLEAN NOT NULL,
    "addresses" TEXT[],
    "facts" JSONB,
    "decode_error" TEXT,
    "chain_error" JSONB,
    "applied_at" TIMESTAMPTZ(6),
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "indexed_transactions_pkey" PRIMARY KEY ("signature")
);

-- CreateTable
CREATE TABLE "indexer_cursors" (
    "address" VARCHAR(44) NOT NULL,
    "latest_signature" VARCHAR(88),
    "latest_slot" BIGINT,
    "discovered_by" VARCHAR(88),
    "updated_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "indexer_cursors_pkey" PRIMARY KEY ("address")
);

-- CreateTable
CREATE TABLE "worker_leases" (
    "name" TEXT NOT NULL,
    "owner" TEXT NOT NULL,
    "expires_at" TIMESTAMPTZ(6) NOT NULL,

    CONSTRAINT "worker_leases_pkey" PRIMARY KEY ("name")
);

-- CreateTable
CREATE TABLE "ServiceReadiness" (
    "id" TEXT NOT NULL,
    "ready" BOOLEAN NOT NULL DEFAULT false,
    "nextProbeAt" TIMESTAMPTZ(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "checkedAt" TIMESTAMPTZ(3),
    "lastError" TEXT,

    CONSTRAINT "ServiceReadiness_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "records_slot_address_idx" ON "records"("slot" DESC, "address" DESC);

-- CreateIndex
CREATE INDEX "records_program_slot_address_idx" ON "records"("program", "slot" DESC, "address" DESC);

-- CreateIndex
CREATE INDEX "flags_record_address_slot_address_idx" ON "flags"("record_address", "slot", "address");

-- CreateIndex
CREATE INDEX "weighings_sampledAt_idx" ON "weighings"("sampledAt" DESC);

-- CreateIndex
CREATE INDEX "indexed_transactions_applied_at_slot_transaction_index_idx" ON "indexed_transactions"("applied_at", "slot", "transaction_index");

-- AddForeignKey
ALTER TABLE "flags" ADD CONSTRAINT "flags_record_address_fkey" FOREIGN KEY ("record_address") REFERENCES "records"("address") ON DELETE RESTRICT ON UPDATE CASCADE;

