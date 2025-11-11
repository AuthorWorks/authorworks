#!/bin/bash

# Script to update Authelia configuration for AuthorWorks public landing page

CONFIG_FILE="/home/l3o/git/homelab/services/authelia/configuration.yml"
BACKUP_FILE="/home/l3o/git/homelab/services/authelia/configuration.yml.backup-$(date +%Y%m%d-%H%M%S)"

echo "Creating backup of Authelia configuration..."
sudo cp "$CONFIG_FILE" "$BACKUP_FILE"

echo "Updating Authelia configuration to allow public landing page..."

# Create temporary file with updated configuration
cat > /tmp/authelia-update.txt << 'EOF'
    # AuthorWorks - public landing page, protected app
    - domain: 'authorworks.leopaska.xyz'
      resources:
        - '^/$'
        - '^/landing.html$'
        - '^/assets/.*$'
        - '^/.*\.(wasm|js|css|ico)$'
      policy: 'bypass'

    - domain: 'authorworks.leopaska.xyz'
      resources:
        - '^/app.*$'
        - '^/api/.*$'
      policy: 'one_factor'
      subject:
        - ['group:admins']
        - ['group:users']

    # Production K3s applications - one factor
    - domain:
        - 'localist.leopaska.xyz'
        - 'potluck.leopaska.xyz'
        - 'ae.leopaska.xyz'
        - 'blink.leopaska.xyz'
        - 'blink.weopaska.xyz'
        - 'hyva.leopaska.xyz'
        - 'ursulai.leopaska.xyz'
        - 'omni.leopaska.xyz'
        - 'api.authorworks.leopaska.xyz'
      policy: 'one_factor'
      subject:
        - ['group:admins']
        - ['group:users']
EOF

# Use sudo to update the file
sudo sed -i '/# Production K3s applications - one factor/,/- authorworks\.leopaska\.xyz/d' "$CONFIG_FILE"
sudo sed -i '/# Production K3s applications - one factor/r /tmp/authelia-update.txt' "$CONFIG_FILE"

echo "Restarting Authelia container..."
AUTHELIA_CONTAINER=$(docker ps --filter name=authelia --format "{{.Names}}" | head -1)
if [ -n "$AUTHELIA_CONTAINER" ]; then
    docker restart "$AUTHELIA_CONTAINER"
    echo "Authelia restarted successfully"
else
    echo "Error: Authelia container not found"
    exit 1
fi

rm /tmp/authelia-update.txt

echo "Configuration updated successfully!"
echo "Backup saved to: $BACKUP_FILE"
EOF

chmod +x /home/l3o/git/production/authorworks/update-authelia-config.sh
