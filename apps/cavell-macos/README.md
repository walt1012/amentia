# Cavell macOS App

This package contains the native macOS shell for `Cavell`.

Initial goals:

- native Intel Mac experience
- workspace and thread navigation
- timeline-centered task execution UI
- runtime bridge over `stdio`

The current package provides the application shell and view structure for Milestone 0.

## Local Runtime Bridge

For local development, point the app at a built runtime executable with:

```bash
export CAVELL_RUNTIME_PATH=/absolute/path/to/cavell-runtime-bin
```

The shell will use that executable when the runtime launch action is triggered.
