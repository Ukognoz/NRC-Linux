#!/bin/sh
# Preserve the desktop settings before isolating the beta launcher's files.
export NRC_HOST_XDG_DATA_HOME="${XDG_DATA_HOME-}"
export NRC_HOST_XDG_CONFIG_HOME="${XDG_CONFIG_HOME-}"
export NRC_HOST_XDG_CACHE_HOME="${XDG_CACHE_HOME-}"
export NRC_HOST_XDG_DATA_DIRS="${XDG_DATA_DIRS-}"
export NRC_HOST_GTK_THEME="${GTK_THEME-}"
export NRC_HOST_GDK_BACKEND="${GDK_BACKEND-}"
export XDG_DATA_HOME="$HOME/.local/share/norisk-beta/data"
export XDG_CONFIG_HOME="$HOME/.local/share/norisk-beta/config"
export XDG_CACHE_HOME="$HOME/.local/share/norisk-beta/cache"
exec "$HOME/.local/opt/norisk-beta/NoRiskClient-Linux.AppImage" "$@"
