# Browser Extension Error Patterns

Patterns for identifying third-party browser extension errors in Sentry stack traces.

## Extension URL Patterns

| Browser      | URL Pattern                              |
| ------------ | ---------------------------------------- |
| Chrome       | `chrome-extension://[extension-id]/`     |
| Firefox      | `moz-extension://[uuid]/`                |
| Safari       | `safari-extension://[bundle-id]-[hash]/` |
| Safari (Web) | `safari-web-extension://[uuid]/`         |
| Edge         | `extension://[extension-id]/`            |

## Injected Script Filenames

These filenames in stack traces indicate extension-injected code:

| Filename           | Common Source                      |
| ------------------ | ---------------------------------- |
| `inpage.js`        | Wallet extensions (MetaMask, etc.) |
| `content.js`       | Generic content scripts            |
| `contentscript.js` | Content scripts                    |
| `inject.js`        | Script injection                   |
| `injected.js`      | Script injection                   |
| `background.js`    | Extension background scripts       |
| `pageScript.js`    | Page-level scripts                 |
| `provider.js`      | Web3 providers                     |
| `ethereum.js`      | Ethereum providers                 |

## Known Extension Error Messages

### Wallet Extensions

```
Cannot read properties of undefined (reading 'ethereum')
Cannot read properties of undefined (reading 'solana')
ethereum is not defined
window.ethereum is undefined
Failed to execute 'postMessage' on 'Window'
```

### General Extensions

```
ResizeObserver loop limit exceeded
ResizeObserver loop completed with undelivered notifications
Extension context invalidated
The message port closed before a response was received
A listener indicated an asynchronous response by returning true
Script error. (no stack trace)
```

### Ad Blockers

```
Failed to load resource: net::ERR_BLOCKED_BY_CLIENT
Blocked by content filter
The resource was blocked by a content blocker
```

### Password Managers

```
Cannot read properties of null (reading 'querySelector')
Unable to find form element
Password field not found
```

## Known Extension Stack Trace Patterns

### MetaMask

```
at MetaMask
at e.request (inpage.js:1:xxxxx)
at Object.request (provider.js:xxx)
```

### Coinbase Wallet

```
at CoinbaseWalletSDK
at CoinbaseWalletProvider
at coinbaseWalletExtension
```

### Phantom (Solana)

```
at PhantomInjectedProvider
at Proxy.request
solana is not defined
```

### WalletConnect

```
at WalletConnect
at Connector.connect
```

### Grammarly

```
at grammarly-desktop-integration
at GrammarlyButton
at grammarly-extension
```

### LastPass

```
at lastpass
at LPContentScriptFeatures
at lpOnLoad
```

### 1Password

```
at 1Password
at onepassword
at op-autofill
```

### Honey

```
at honey
at HoneyContainer
at PayPal Honey
```

### uBlock Origin / AdBlock

```
at uBlock
at AdBlock
at cosmetic-filter
```

## Sentry Event Fields to Check

1. `exception.values[].stacktrace.frames[].filename` - Extension URLs
2. `exception.values[].stacktrace.frames[].absPath` - Full path with `extension://`
3. `exception.values[].stacktrace.frames[].module` - Extension modules
4. `exception.values[].value` - Error message against known patterns
5. `tags.browser.name` - Some extension errors are browser-specific

## False Positive Considerations

Some errors that look like extension errors but might be valid:

- **Web3 errors without extension stack** - May be application code mishandling missing providers
- **ResizeObserver in application code** - Check if the observer is yours
- **Network errors** - Could be user connectivity OR application API issues

Always examine the full stack trace before categorizing.
