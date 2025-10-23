# Android Permissions for CloudHost TUI (Termux)

This document explains the Android permissions required for CloudHost TUI when running in Termux.

## Why No C++ Wrapper?

**Termux Approach** (What we're using):
```
Termux (Linux environment)
    ↓ (direct execution)
Rust Binary (cloudhost-tui)
```

**Native Android App Approach** (What C++ wrapper is for):
```
Android App (Java)
    ↓ (JNI)
C++ Wrapper
    ↓ (FFI)
Rust Library
```

Since we're targeting **Termux** (not a native Android app), we **DON'T need** the C++ wrapper!

## Required Permissions

### File System Permissions
- **`READ_EXTERNAL_STORAGE`** - Read files from external storage
- **`WRITE_EXTERNAL_STORAGE`** - Write files to external storage  
- **`MANAGE_EXTERNAL_STORAGE`** - Full access to external storage (Android 11+)

### Network Permissions
- **`INTERNET`** - Access the internet for cloud functionality
- **`ACCESS_NETWORK_STATE`** - Check network connectivity

### Media Permissions (Android 13+)
- **`READ_MEDIA_IMAGES`** - Read image files
- **`READ_MEDIA_VIDEO`** - Read video files
- **`READ_MEDIA_AUDIO`** - Read audio files

## Permission Handling in Termux

### Automatic Permission Request
Termux automatically handles most permissions:

```bash
# Termux automatically requests permissions when needed
# No Java code required!
```

### Manual Permission Granting
Some permissions require manual user action:

1. **MANAGE_EXTERNAL_STORAGE** - User must grant "All files access" in Settings
2. **Media permissions** - User must grant access to media files

## File Operations

### Supported Operations
- ✅ **Read files** from cloud folders
- ✅ **Write files** to cloud folders  
- ✅ **Delete files** (permanently on Android)
- ✅ **Create directories** for organizing files
- ✅ **List directory contents**

### Platform-Specific Behavior
- **Desktop**: Files moved to OS trash (restorable)
- **Android**: Files permanently deleted (no trash overhead)

## Troubleshooting

### Permission Denied Errors
If you get permission denied errors:

1. **Check app permissions** in Android Settings
2. **Grant "All files access"** for MANAGE_EXTERNAL_STORAGE
3. **Restart the app** after granting permissions
4. **Check Termux permissions** if using Termux

### Common Issues
- **"Permission denied"** - Grant storage permissions
- **"Network error"** - Grant internet permission
- **"File not found"** - Check storage access permissions

## Security Considerations

### File Access
- App only accesses files in designated cloud folders
- No access to system files or other apps' data
- User controls which directories are accessible

### Network Security
- All network communication is encrypted (HTTPS)
- Authentication required for cloud access
- No data sent to external servers without user consent

## Development Notes

### Building for Android
```bash
# Build with proper permissions
./build-android.sh

# Or manually
cargo build --release --target aarch64-linux-android
```

### Testing Permissions
```bash
# Check if permissions are granted
adb shell dumpsys package com.cloudhost.tui | grep permission

# Test file operations
adb shell run-as com.cloudhost.tui ls /data/data/com.cloudhost.tui/files
```

## References
- [Android Storage Permissions](https://developer.android.com/training/data-storage)
- [Scoped Storage](https://developer.android.com/training/data-storage#scoped-storage)
- [Termux Permissions](https://wiki.termux.com/wiki/Internal_and_external_storage)
