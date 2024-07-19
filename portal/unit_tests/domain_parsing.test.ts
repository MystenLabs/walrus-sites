import { describe, it, expect, test } from 'vitest'
import { getDomain } from '../src/helpers/domain_parsing.ts'

describe('getDomain', () => {
  test('https://example.com -> example.com', () => {
    const domain = getDomain('https://example.com')
    expect(domain).toEqual('example.com')
  })

  test('https://suinsname.localhost:8080 -> localhost', () => {
    const domain = getDomain('https://suinsname.localhost:8080')
    expect(domain).toEqual('localhost')
  })

  test('https://suinsname.subname.localhost:8080 -> localhost', () => {
    const domain = getDomain('https://suinsname.subname.localhost:8080')
    expect(domain).toEqual('localhost')
  })

  test('https://flatland.walrus.site/ -> walrus.site', () => {
    const domain = getDomain('https://flatland.walrus.site/')
    expect(domain).toEqual('walrus.site')
  })

  test('https://4snh0c0o7quicfzokqpsmuchtgitnukme1q680o1s1nfn325sr.walrus.site/ -> walrus.site', () => {
    const domain = getDomain('https://4snh0c0o7quicfzokqpsmuchtgitnukme1q680o1s1nfn325sr.walrus.site/')
    expect(domain).toEqual('walrus.site')
  })

  test('https://4snh0c0o7quicfzokqpsmuchtgitnukme1q680o1s1nfn325sr.portalname.co.uk/ -> portalname.co.uk', () => {
    const domain = getDomain('https://4snh0c0o7quicfzokqpsmuchtgitnukme1q680o1s1nfn325sr.portalname.co.uk/')
    expect(domain).toEqual('portalname.co.uk')
  })

  test('https://suinsname.subname.portalname.co.uk/ -> portalname.co.uk', () => {
    const domain = getDomain('https://suinsname.subname.portalname.co.uk/')
    expect(domain).toEqual('portalname.co.uk')
  })

  test('https://suinsname.subname.anothersubname.portalname.co.uk/ -> portalname.co.uk', () => {
    const domain = getDomain('https://suinsname.subname.anothersubname.portalname.co.uk/')
    expect(domain).toEqual('portalname.co.uk')
  })
})
