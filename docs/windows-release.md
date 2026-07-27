# Windows release security

RID uses the standard Tauri NSIS and WiX bundlers. Release builds are produced
on a clean GitHub-hosted Windows runner, smoke-tested, hashed, and accompanied
by GitHub build-provenance attestations.

## Current signing status

RID does not currently have an Authenticode code-signing certificate. A
self-signed certificate is intentionally not used because Windows does not
trust it and it does not establish the publisher's identity.

To remove the unknown-publisher warning, obtain a Windows code-signing
certificate or an Azure Artifact Signing profile. Never commit a PFX file,
private key, certificate password, or cloud credential to this repository.

Tauri supports either:

- a certificate installed in the Windows certificate store, configured with
  `bundle.windows.certificateThumbprint`, `digestAlgorithm`, and `timestampUrl`;
- a remote or hardware-backed signer configured through
  `bundle.windows.signCommand`.

After signing is configured, run:

```powershell
npm run tauri:build
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts/verify-windows-release.ps1 -RequireSignature
```

The verification must report `Valid` for `rid.exe`, the NSIS installer, and the
MSI package before publishing.

## Verify a downloaded release

Compare a download with `SHA256SUMS.txt` from the same GitHub Release:

```powershell
Get-FileHash .\RID_0.1.1_x64-setup.exe -Algorithm SHA256
```

For releases built by GitHub Actions, verify the signed provenance statement:

```powershell
gh attestation verify .\RID_0.1.1_x64-setup.exe -R itoyohane/RID
```

An attestation links the binary to its repository, workflow, and commit. It is
not a replacement for Authenticode or malware scanning.

## Silent installation

The NSIS setup executable uses Tauri's `currentUser` install mode and supports
the standard, case-sensitive `/S` switch:

```powershell
RID_0.1.1_x64-setup.exe /S
```

To override the destination, `/D` must be the final argument:

```powershell
RID_0.1.1_x64-setup.exe /S /D=C:\Tools\RID
```

The MSI package uses standard Windows Installer arguments:

```powershell
msiexec.exe /i RID_0.1.1_x64_en-US.msi /quiet /norestart
```

The GitHub workflow performs an actual silent install and uninstall on an
ephemeral Windows runner for every relevant pull request and release tag.

## False-positive response

If Microsoft Defender incorrectly detects a release:

1. Keep the affected binary unchanged.
2. Record its SHA-256 digest and the exact detection name.
3. Submit the individual file as a software developer at the
   [Microsoft Security Intelligence submission portal](https://www.microsoft.com/wdsi/filesubmission).
4. Keep the submission ID with the release notes until Microsoft responds.
