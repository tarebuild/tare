-- The attest fee each record paid to the launch wallet.
ALTER TABLE "records" ADD COLUMN "feeLamports" BIGINT NOT NULL DEFAULT 0;
