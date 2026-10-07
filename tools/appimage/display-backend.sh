# Sourced by the packaged GTK AppRun hook, before GTK initializes.
# A caller's nonempty backend selection always wins. GTK handles connection
# attempts/fallback; a session variable is a hint, not proof of a live display.
if [ -z "${GDK_BACKEND:-}" ]; then
    if [ -n "${WAYLAND_DISPLAY:-}" ] || [ "${XDG_SESSION_TYPE:-}" = wayland ]; then
        export GDK_BACKEND=wayland,x11
    else
        export GDK_BACKEND=x11
    fi
fi

# WebKitGTK's DMABUF path failed on the tested native Wayland setup (KL-158).
# Default to its non-DMABUF renderer only when Wayland is permitted. Preserve
# explicit settings, including 0 and the empty string, for driver diagnostics.
case ",$GDK_BACKEND," in
    *,wayland,*|*,\*,*)
        if [ "${WEBKIT_DISABLE_DMABUF_RENDERER+x}" != x ]; then
            export WEBKIT_DISABLE_DMABUF_RENDERER=1
        fi
        ;;
esac
