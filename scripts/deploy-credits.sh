#!/bin/bash
# AuthorWorks Credit System Deployment Script
# Deploys credit system to local or K3s PostgreSQL

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}==================================================================${NC}"
echo -e "${GREEN}  AuthorWorks Credit System Deployment${NC}"
echo -e "${GREEN}==================================================================${NC}"
echo ""

# Function to print colored messages
info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check for required files
if [ ! -f "$SCRIPT_DIR/migrations/001_add_credit_system.sql" ]; then
    error "Migration file not found: $SCRIPT_DIR/migrations/001_add_credit_system.sql"
    exit 1
fi

# Detect environment
echo "Select deployment environment:"
echo "1) Local PostgreSQL (Docker)"
echo "2) K3s PostgreSQL (neon-postgres-leopaska)"
echo "3) Custom (manual connection string)"
read -p "Enter choice [1-3]: " ENV_CHOICE

case $ENV_CHOICE in
    1)
        info "Deploying to Local PostgreSQL..."
        DB_HOST="localhost"
        DB_PORT="5432"
        DB_USER="postgres"
        read -sp "Enter PostgreSQL password: " DB_PASS
        echo ""
        DB_NAME="authorworks"
        DB_URL="postgresql://$DB_USER:$DB_PASS@$DB_HOST:$DB_PORT/$DB_NAME"
        ;;
    2)
        info "Deploying to K3s PostgreSQL..."
        DB_HOST="neon-postgres-leopaska"
        DB_PORT="5432"
        DB_USER="postgres"
        DB_PASS="postgresstrongpassword123"
        DB_NAME="authorworks"
        DB_URL="postgresql://$DB_USER:$DB_PASS@$DB_HOST:$DB_PORT/$DB_NAME"

        info "Will execute inside K3s cluster via kubectl..."
        ;;
    3)
        info "Custom connection..."
        read -p "Enter full PostgreSQL connection string: " DB_URL
        ;;
    *)
        error "Invalid choice"
        exit 1
        ;;
esac

echo ""
info "Connection: $DB_URL"
echo ""

# Confirm before proceeding
read -p "Proceed with migration? [y/N]: " CONFIRM
if [[ ! $CONFIRM =~ ^[Yy]$ ]]; then
    warn "Deployment cancelled"
    exit 0
fi

echo ""
info "Starting migration..."
echo ""

# Execute migration based on environment
if [ "$ENV_CHOICE" == "2" ]; then
    # K3s deployment
    info "Creating temporary pod for migration..."

    # Copy migration file to a configmap
    kubectl create configmap credit-migration \
        --from-file=migration.sql="$SCRIPT_DIR/migrations/001_add_credit_system.sql" \
        -n authorworks \
        --dry-run=client -o yaml | kubectl apply -f -

    # Run migration in a temporary pod
    kubectl run psql-migration \
        --rm -i --restart=Never \
        --image=postgres:15-alpine \
        --namespace=authorworks \
        --overrides="{
            \"spec\": {
                \"containers\": [{
                    \"name\": \"psql\",
                    \"image\": \"postgres:15-alpine\",
                    \"command\": [\"/bin/sh\", \"-c\"],
                    \"args\": [\"psql \$DATABASE_URL < /migration/migration.sql\"],
                    \"env\": [{
                        \"name\": \"DATABASE_URL\",
                        \"value\": \"$DB_URL\"
                    }],
                    \"volumeMounts\": [{
                        \"name\": \"migration\",
                        \"mountPath\": \"/migration\"
                    }]
                }],
                \"volumes\": [{
                    \"name\": \"migration\",
                    \"configMap\": {
                        \"name\": \"credit-migration\"
                    }
                }]
            }
        }"

    info "Cleaning up configmap..."
    kubectl delete configmap credit-migration -n authorworks

else
    # Local or custom deployment
    info "Applying migration via psql..."

    PGPASSWORD="$DB_PASS" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" \
        -f "$SCRIPT_DIR/migrations/001_add_credit_system.sql"
fi

if [ $? -eq 0 ]; then
    echo ""
    info "Migration completed successfully!"
    echo ""

    # Verify migration
    info "Verifying migration..."

    if [ "$ENV_CHOICE" == "2" ]; then
        # K3s verification
        kubectl run psql-verify \
            --rm -i --restart=Never \
            --image=postgres:15-alpine \
            --namespace=authorworks \
            --command -- \
            psql "$DB_URL" -c "SELECT name, credit_amount, price_cents FROM subscriptions.credit_packages ORDER BY sort_order;"
    else
        # Local verification
        PGPASSWORD="$DB_PASS" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" \
            -c "SELECT name, credit_amount, price_cents FROM subscriptions.credit_packages ORDER BY sort_order;"
    fi

    echo ""
    info "Credit packages created:"
    echo "  - Starter Pack:   1,000 credits @ $9.99"
    echo "  - Writer Pack:    5,000 credits @ $39.99"
    echo "  - Author Pack:   15,000 credits @ $99.99"
    echo "  - Publisher Pack: 50,000 credits @ $299.99"
    echo ""

    echo -e "${GREEN}==================================================================${NC}"
    echo -e "${GREEN}  Next Steps:${NC}"
    echo -e "${GREEN}==================================================================${NC}"
    echo "1. Update Stripe configuration in .env file"
    echo "2. Create products in Stripe Dashboard"
    echo "3. Update credit_packages table with Stripe price IDs"
    echo "4. Restart services to pick up new environment variables"
    echo "5. Test credit purchase flow"
    echo ""
    echo "See docs/PRODUCTION_SETUP.md for detailed instructions"
    echo ""

else
    error "Migration failed!"
    exit 1
fi
