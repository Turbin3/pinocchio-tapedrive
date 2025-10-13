# ELFs Directory

This directory contains external program binaries needed for testing.

## Required Programs

### Metadata Program (metadata.so)

The tests require the Metaplex Token Metadata program binary.

To download it:

```bash
solana program dump --url mainnet-beta metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s metadata.so
```

Or use the one from the native tape program tests:

```bash
cp ../../../tape/program/tests/elfs/metadata.so .
```

## File List

- `metadata.so` - Metaplex Token Metadata program (Program ID: metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s)

