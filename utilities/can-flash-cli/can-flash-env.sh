# Source from Bash or Zsh to enable can. Build separately with cargo build --release.
if [ -n "${BASH_VERSION:-}" ]; then
    _can_flash_env_file="${BASH_SOURCE[0]}"
elif [ -n "${ZSH_VERSION:-}" ]; then
    _can_flash_env_file="${(%):-%x}"
else
    printf '%s\n' 'Source can-flash-env.sh from Bash or Zsh.' >&2
    return 1
fi

_CAN_FLASH_TOOL_DIR="$(cd -- "$(dirname -- "$_can_flash_env_file")" && pwd -P)" || return 1
unset _can_flash_env_file

_can_flash_host="$(cd -- "$_CAN_FLASH_TOOL_DIR" && rustc -vV)" || return 1
_can_flash_host="$(printf '%s\n' "$_can_flash_host" | sed -n 's/^host: //p')"
if [ -z "$_can_flash_host" ]; then
    printf '%s\n' 'Cannot determine the Rust host target.' >&2
    return 1
fi
_CAN_FLASH_BINARY="$_CAN_FLASH_TOOL_DIR/target/$_can_flash_host/release/can"
unset _can_flash_host

can() {
    if [ ! -x "$_CAN_FLASH_BINARY" ]; then
        printf 'CLI is not built. Run cargo build --release from: %s\n' "$_CAN_FLASH_TOOL_DIR" >&2
        return 1
    fi
    # Preserve the caller's directory for firmware package and ELF discovery.
    "$_CAN_FLASH_BINARY" "$@"
}
