-- Every record and crank transaction before this paid or was signed by the launch wallet.
ALTER TABLE "records" ADD COLUMN "feeRecipient" VARCHAR(44) NOT NULL DEFAULT '';
UPDATE "records" SET "feeRecipient" = 'EvycjhbgcQDVN92xpPsdy6oLKezCcDiykg96g8ZXZDe9';
ALTER TABLE "burns" ADD COLUMN "wallet" TEXT NOT NULL DEFAULT '';
UPDATE "burns" SET "wallet" = 'EvycjhbgcQDVN92xpPsdy6oLKezCcDiykg96g8ZXZDe9';
