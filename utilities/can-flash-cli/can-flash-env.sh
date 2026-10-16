# Source this file from Bash or Zsh to make the `can` command available.
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

can() {
    local can_flash_target
    if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
        printf '%s\n' 'The can command requires Rust and Cargo.' >&2
        return 1
    fi

    can_flash_target="$(cd -- "$_CAN_FLASH_TOOL_DIR" && rustc -vV)" || return 1
    can_flash_target="$(printf '%s\n' "$can_flash_target" | sed -n 's/^host: //p')"
    if [ -z "$can_flash_target" ]; then
        printf '%s\n' 'Cannot determine the Rust host target.' >&2
        return 1
    fi

    # Build for this computer, independently of the application's Cargo config.
    (
        cd -- "$_CAN_FLASH_TOOL_DIR" || exit 1
        cargo build --locked --bin can \
            --target "$can_flash_target" \
            --target-dir "$_CAN_FLASH_TOOL_DIR/target"
    ) || return $?

    # Keep the caller's directory for ELF discovery and relative file paths.
    "$_CAN_FLASH_TOOL_DIR/target/$can_flash_target/debug/can" "$@"
}
