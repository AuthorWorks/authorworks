export async function resolve(specifier, context, nextResolve) {
  try {
    return await nextResolve(specifier, context)
  } catch (err) {
    if (specifier.startsWith('.') && !specifier.endsWith('.node')) {
      for (const candidate of [specifier + '.ts', specifier + '.tsx']) {
        try {
          return await nextResolve(candidate, context)
        } catch {
          /* try next */
        }
      }
    }
    throw err
  }
}
