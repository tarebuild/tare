-- The crank's journal.
CREATE TABLE "KeeperTransaction" (
    "key" TEXT NOT NULL,
    "state" TEXT NOT NULL,
    "signature" TEXT NOT NULL,
    "signedBase64" TEXT NOT NULL,
    "blockhash" TEXT NOT NULL,
    "lastValidBlockHeight" BIGINT NOT NULL,
    "contextSlot" BIGINT NOT NULL,
    "error" TEXT,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,
    CONSTRAINT "KeeperTransaction_pkey" PRIMARY KEY ("key")
);
CREATE UNIQUE INDEX "KeeperTransaction_signature_key" ON "KeeperTransaction"("signature");
CREATE INDEX "KeeperTransaction_state_createdAt_idx" ON "KeeperTransaction"("state", "createdAt");
