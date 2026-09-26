# Nova Development Guide

## Environment Setup

### Linux Desktop Development
Ensure you have the latest Rust stable toolchain and Node.js:

```bash
rustup default stable
rustup component add rust-std cargo
```

### Building the Core Daemon

```bash
cargo build --workspace
cargo test --workspace
```

### Running the Linux Desktop Client

```bash
cd apps/linux
npm run build
npx vite --port 3000
```

### Developing Android Components

Open `apps/android` in Android Studio or compile with Gradle:

```bash
cd apps/android
./gradlew assembleDebug
```
