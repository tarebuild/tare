# Tare backend services

Node 22, TypeScript, Hono, Zod and Prisma 7 over PostgreSQL. One package runs the API and registry indexer against the same database. The API samples deployed programs on request; it never polls mainnet.

## Run

From `backend/` on Windows:

```powershell
npm.cmd ci
npm.cmd run build
npm.cmd run fixture
```

The fixture command serves `127.0.0.1:7810`. It needs no RPC or database. Set `PORT` for a separate instance. Existing API processes keep running during a build; restart the intended service to load a new build.

For real services, set `MODE=chain` and `DATABASE_URL`, then:

```powershell
$env:MODE='chain'
$env:PORT='7812'
npm.cmd start
```

In another terminal, with its own port:

```powershell
$env:MODE='chain'
$env:PORT='7813'
npm.cmd run start:indexer
```

Both start scripts apply migrations before launching the role. Build runs `prisma generate` without contacting PostgreSQL. Stop with Ctrl+C; the worker stops indexing and releases its leader lease. `npm.cmd run dev` watches API source. `npm.cmd test` runs only the two approved suites: compute grouping and registry decoding.
