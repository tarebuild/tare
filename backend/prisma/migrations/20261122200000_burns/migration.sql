CREATE TABLE "burns" (
    "signature" TEXT NOT NULL,
    "key" TEXT NOT NULL,
    "kind" TEXT NOT NULL,
    "spentLamports" BIGINT NOT NULL,
    "burned" BIGINT NOT NULL,
    "decimals" INTEGER NOT NULL,
    "slot" BIGINT NOT NULL,
    "at" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "burns_pkey" PRIMARY KEY ("signature")
);

CREATE UNIQUE INDEX "burns_key_key" ON "burns"("key");
CREATE INDEX "burns_slot_idx" ON "burns"("slot");
