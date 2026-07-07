#!/usr/bin/env bash
set -euo pipefail

APP_NAME="mqtt-locker"
SERVICE_FILE="${APP_NAME}.service"
CONFIG_DIR="${HOME}/.config/${APP_NAME}"
ENV_FILE="${CONFIG_DIR}/env"
SYSTEMD_USER_DIR="${HOME}/.config/systemd/user"

echo "Installing ${APP_NAME} with cargo..."
cargo install --path .

echo "Creating config directories..."
mkdir -p "${CONFIG_DIR}"
mkdir -p "${SYSTEMD_USER_DIR}"

if [[ ! -f "${ENV_FILE}" ]]; then
    echo "Creating example env file at ${ENV_FILE}"
    cat > "${ENV_FILE}" <<'EOF'
MQTT_BROKER_HOST=192.168.0.10
MQTT_BROKER_PORT=1883
MQTT_BROKER_USERNAME=
MQTT_BROKER_PASSWORD=
LOCK_PROG=swaylock
LOCK_PROG_ARGS=
HOME_ASSISTANT_AREA=Office
EOF

    echo
    echo "Edit ${ENV_FILE} before starting the service."
else
    echo "Keeping existing env file: ${ENV_FILE}"
fi

echo "Installing systemd user service..."
cp "${SERVICE_FILE}" "${SYSTEMD_USER_DIR}/${SERVICE_FILE}"

echo "Reloading systemd user daemon..."
systemctl --user daemon-reload

echo "Enabling ${SERVICE_FILE}..."
systemctl --user enable "${SERVICE_FILE}"

echo
echo "Installed. Next steps:"
echo "  1. Edit ${ENV_FILE}"
echo "  2. Start the service:"
echo "       systemctl --user start ${SERVICE_FILE}"
