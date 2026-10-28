import { z } from 'zod'

// A response schema as its reader parses it: objects keep keys they don't know, enums keep values
// they don't know, and a union keyed by a discriminator accepts a variant it doesn't know. The server
// validates with the strict schemas; a reader built against an older contract keeps working when the
// API adds a field, a state or an enum value.

const opened = new WeakMap<z.ZodType, z.ZodType>()

export function open<S extends z.ZodType>(schema: S): S {
  let result = opened.get(schema)
  if (!result) {
    result = build(schema)
    opened.set(schema, result)
  }
  return result as S
}

function build(schema: z.ZodType): z.ZodType {
  const def = schema._zod.def as any
  switch (def.type) {
    case 'object': {
      const shape = Object.fromEntries(Object.entries(def.shape as Record<string, z.ZodType>).map(([key, value]) => [key, open(value)]))
      return schema.clone({ ...def, shape, catchall: z.unknown() })
    }
    case 'array':
      return schema.clone({ ...def, element: open(def.element) })
    case 'tuple':
      return schema.clone({ ...def, items: def.items.map(open), rest: def.rest ? open(def.rest) : def.rest })
    case 'record':
      return schema.clone({ ...def, valueType: open(def.valueType) })
    case 'union': {
      const union = schema.clone({ ...def, options: def.options.map(open) })
      return def.discriminator ? z.union([union, z.looseObject({ [def.discriminator]: z.string() })]) : union
    }
    case 'optional':
    case 'nullable':
    case 'default':
    case 'prefault':
    case 'readonly':
      return schema.clone({ ...def, innerType: open(def.innerType) })
    case 'pipe':
      return schema.clone({ ...def, in: open(def.in), out: open(def.out) })
    case 'enum':
      return z.union([schema, z.string()])
    default:
      return schema
  }
}
