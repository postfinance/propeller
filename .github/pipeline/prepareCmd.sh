#!/usr/bin/env sh

set -ex

calculate_hash() {
    file="$1"
    algorithm="${2:-md5}"  # Default to MD5 if no algorithm specified

    if [ ! -f "$file" ]; then
        echo "Error: File not found: $file" >&2
        return 1
    fi

    case "$algorithm" in
        md5)
            hash_cmd="md5sum"
            ;;
        sha256)
            hash_cmd="sha256sum"
            ;;
        *)
            echo "Error: Unsupported hash algorithm: $algorithm" >&2
            return 1
            ;;
    esac

    $hash_cmd "$file" | awk '{print $1}' > "${file}.${algorithm}"
    echo "Hash calculated and saved to ${file}.${algorithm}"
}

release_version=$1

cargo bump "${release_version}"
cargo build --release & cross build --target x86_64-pc-windows-gnu --release

# Calculate MD5 Hashes
calculate_hash "target/release/propeller"
calculate_hash "target/x86_64-pc-windows-gnu/release/propeller.exe"
