// The docs tree: groups, their chapters, and each chapter's pages. A plain module, so server and
// client components both read it. `path` is the route under /docs ('' is the overview).

export type DocNode = { path: string; title: string; says: string; children?: DocNode[] }

export const GROUPS: { label: string; pages: DocNode[] }[] = [
  {
    label: 'Get started',
    pages: [
      {
        path: '',
        title: 'Overview',
        says: 'What Tare is, in one page.',
        children: [
          { path: 'weights', title: 'The three weights', says: 'Gross, tare and net, and how each is measured.' },
          { path: 'identical', title: 'What stays identical', says: 'Everything a client, an indexer or an account can observe.' },
        ],
      },
      {
        path: 'install',
        title: 'Install',
        says: 'The binary, what it needs, and its environment.',
        children: [
          { path: 'install/requirements', title: 'Requirements', says: 'What has to be on the machine.' },
          { path: 'install/environment', title: 'Environment', says: 'The variables the CLI reads.' },
        ],
      },
    ],
  },
  {
    label: 'Commands',
    pages: [
      {
        path: 'build',
        title: 'Build',
        says: 'tare build: two programs from one source.',
        children: [
          { path: 'build/outputs', title: 'What it writes', says: 'Every file under target/.' },
          { path: 'build/substitution', title: 'Dependency substitution', says: 'How Tare takes the place of anchor-lang.' },
          { path: 'build/language', title: 'The Anchor language', says: 'Entrypoint, dispatch, accounts, CPI, errors.' },
          { path: 'build/container', title: 'The container', says: 'The pinned image both builds run in.' },
          { path: 'build/deploy', title: 'Deploy the Tare build', says: 'An upgrade in place, at the same address.' },
        ],
      },
      {
        path: 'weigh',
        title: 'Weigh',
        says: 'tare weigh: the three weights by instruction.',
        children: [
          { path: 'weigh/reading', title: 'Reading a weighing', says: 'Columns, medians, unmeasured instructions.' },
          { path: 'weigh/file', title: 'weigh.json', says: 'The weighing as a document.' },
          { path: 'weigh/deployed', title: 'A deployed program', says: 'Weighing by address on the site.' },
        ],
      },
      {
        path: 'prove',
        title: 'Prove',
        says: 'tare prove: both builds, run side by side.',
        children: [
          { path: 'prove/corpora', title: 'The three corpora', says: 'Own tests, generated, mainnet.' },
          { path: 'prove/comparison', title: 'What is compared', says: 'After every transaction.' },
          { path: 'prove/divergence', title: 'A divergence', says: 'What the command prints and writes.' },
          { path: 'prove/report', title: 'proof.json', says: 'The report a record commits to.' },
        ],
      },
      {
        path: 'check',
        title: 'Attest and check',
        says: 'Records on the registry.',
        children: [
          { path: 'check/attest', title: 'tare attest', says: 'Publish a proof.' },
          { path: 'check/record', title: 'Reading a record', says: 'What a record holds and where it is shown.' },
          { path: 'check/check', title: 'tare check', says: 'Rebuild and rerun a record.' },
          { path: 'check/flag', title: 'tare flag', says: 'File an input the two builds differ on.' },
        ],
      },
    ],
  },
  {
    label: 'Reference',
    pages: [
      {
        path: 'compatibility',
        title: 'Compatibility',
        says: 'Anchor versions and constructs.',
        children: [{ path: 'compatibility/versions', title: 'Anchor versions', says: 'The pinned version lines.' }],
      },
      {
        path: 'bytecode',
        title: 'Bytecode stage',
        says: 'tare-opt, over the linked program.',
        children: [
          { path: 'bytecode/passes', title: 'The passes', says: 'What each one rewrites.' },
          { path: 'bytecode/checks', title: 'How rewrites are checked', says: 'The verifier, the VM, the corpus.' },
        ],
      },
      {
        path: 'registry',
        title: 'Registry program',
        says: 'tare_registry, on chain.',
        children: [
          { path: 'registry/accounts', title: 'Accounts', says: 'Config, Record, Flag.' },
          { path: 'registry/instructions', title: 'Instructions', says: 'init_config, set_config, attest, flag.' },
        ],
      },
      { path: 'token', title: 'Token', says: 'TARE and the attest fee.' },
    ],
  },
]

export const docHref = (path: string) => (path ? `/docs/${path}` : '/docs')

/** The ten chapters, in order. */
export const CHAPTERS: DocNode[] = GROUPS.flatMap((g) => g.pages)

/** Every page in reading order: each chapter, then its pages. */
export const FLAT: (DocNode & { chapter: DocNode; group: string })[] = GROUPS.flatMap((g) =>
  g.pages.flatMap((c) => [{ ...c, chapter: c, group: g.label }, ...(c.children ?? []).map((p) => ({ ...p, chapter: c, group: g.label }))]),
)

export const findDoc = (path: string) => FLAT.find((d) => d.path === path)

/** The chapters as the footer lists them. */
export const DOCS = CHAPTERS.map((c) => ({ slug: c.path, title: c.title, says: c.says }))
