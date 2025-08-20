#!/bin/bash

# RS Helper Deployment Script
# Usage: ./deploy.sh [production|staging|local]

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
APP_NAME="rs-helper"
DOCKER_REGISTRY="your-registry.com"
DOCKER_IMAGE="${DOCKER_REGISTRY}/${APP_NAME}"

# Default environment
ENVIRONMENT="${1:-local}"

# Functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
    fi
    
    # Check Docker Compose
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed"
    fi
    
    # Check Git
    if ! command -v git &> /dev/null; then
        log_error "Git is not installed"
    fi
    
    log_success "All prerequisites met"
}

# Build application
build_app() {
    log_info "Building application..."
    
    # Get current git hash for versioning
    GIT_HASH=$(git rev-parse --short HEAD)
    GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
    BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    log_info "Git Hash: ${GIT_HASH}"
    log_info "Git Branch: ${GIT_BRANCH}"
    log_info "Build Date: ${BUILD_DATE}"
    
    # Build Docker image with labels
    docker build \
        --label "git.hash=${GIT_HASH}" \
        --label "git.branch=${GIT_BRANCH}" \
        --label "build.date=${BUILD_DATE}" \
        --tag "${DOCKER_IMAGE}:${GIT_HASH}" \
        --tag "${DOCKER_IMAGE}:latest" \
        .
    
    log_success "Application built successfully"
}

# Run tests
run_tests() {
    log_info "Running tests..."
    
    # Run Rust tests
    cargo test --release
    
    # Run integration tests if available
    if [ -f "tests/integration_tests.sh" ]; then
        bash tests/integration_tests.sh
    fi
    
    log_success "All tests passed"
}

# Deploy to local environment
deploy_local() {
    log_info "Deploying to local environment..."
    
    # Check if .env file exists
    if [ ! -f ".env" ]; then
        log_warning ".env file not found, copying from .env.example"
        cp .env.example .env
    fi
    
    # Stop existing containers
    docker-compose down
    
    # Start new containers
    docker-compose up -d
    
    # Wait for health check
    log_info "Waiting for application to be healthy..."
    sleep 5
    
    # Check health
    if curl -f http://localhost:3000/healthz > /dev/null 2>&1; then
        log_success "Application is healthy"
        log_info "Application URL: http://localhost:3000"
        log_info "Metrics URL: http://localhost:9090/metrics"
        
        if [ -f "docker-compose.yml" ] && grep -q "prometheus" docker-compose.yml; then
            log_info "Prometheus URL: http://localhost:9091"
            log_info "Grafana URL: http://localhost:3001 (admin/admin)"
        fi
    else
        log_error "Health check failed"
    fi
}

# Deploy to staging environment
deploy_staging() {
    log_info "Deploying to staging environment..."
    
    # Push image to registry
    docker push "${DOCKER_IMAGE}:${GIT_HASH}"
    docker push "${DOCKER_IMAGE}:staging"
    
    # Deploy to staging server (example using SSH)
    STAGING_HOST="${STAGING_HOST:-staging.example.com}"
    STAGING_USER="${STAGING_USER:-deploy}"
    
    ssh "${STAGING_USER}@${STAGING_HOST}" << EOF
        cd /opt/${APP_NAME}
        docker-compose -f docker-compose.staging.yml pull
        docker-compose -f docker-compose.staging.yml up -d --force-recreate
        docker-compose -f docker-compose.staging.yml ps
EOF
    
    log_success "Deployed to staging environment"
}

# Deploy to production environment
deploy_production() {
    log_info "Deploying to production environment..."
    
    # Confirm production deployment
    read -p "Are you sure you want to deploy to PRODUCTION? (yes/no): " confirm
    if [ "$confirm" != "yes" ]; then
        log_warning "Production deployment cancelled"
        exit 0
    fi
    
    # Create backup of current deployment
    log_info "Creating backup of current deployment..."
    BACKUP_TAG="backup-$(date +%Y%m%d-%H%M%S)"
    docker tag "${DOCKER_IMAGE}:production" "${DOCKER_IMAGE}:${BACKUP_TAG}" || true
    
    # Push new image
    docker tag "${DOCKER_IMAGE}:${GIT_HASH}" "${DOCKER_IMAGE}:production"
    docker push "${DOCKER_IMAGE}:production"
    docker push "${DOCKER_IMAGE}:${BACKUP_TAG}" || true
    
    # Deploy to production servers (example with multiple servers)
    PROD_HOSTS="${PROD_HOSTS:-prod1.example.com,prod2.example.com}"
    PROD_USER="${PROD_USER:-deploy}"
    
    IFS=',' read -ra HOSTS <<< "$PROD_HOSTS"
    for host in "${HOSTS[@]}"; do
        log_info "Deploying to ${host}..."
        
        ssh "${PROD_USER}@${host}" << EOF
            cd /opt/${APP_NAME}
            
            # Pull new image
            docker pull ${DOCKER_IMAGE}:production
            
            # Blue-green deployment
            docker-compose -f docker-compose.prod.yml up -d --no-deps --scale app=2 app
            sleep 10
            
            # Health check
            if curl -f http://localhost:3000/healthz; then
                docker-compose -f docker-compose.prod.yml up -d --remove-orphans
                echo "Deployment successful"
            else
                docker-compose -f docker-compose.prod.yml down
                docker tag ${DOCKER_IMAGE}:${BACKUP_TAG} ${DOCKER_IMAGE}:production
                docker-compose -f docker-compose.prod.yml up -d
                echo "Deployment failed, rolled back"
                exit 1
            fi
EOF
    done
    
    log_success "Deployed to production environment"
}

# Rollback deployment
rollback() {
    log_warning "Rolling back deployment..."
    
    read -p "Enter backup tag to rollback to: " backup_tag
    
    if [ -z "$backup_tag" ]; then
        log_error "Backup tag is required"
    fi
    
    # Check if backup exists
    if ! docker images | grep -q "${DOCKER_IMAGE}:${backup_tag}"; then
        log_error "Backup tag ${backup_tag} not found"
    fi
    
    # Perform rollback
    docker tag "${DOCKER_IMAGE}:${backup_tag}" "${DOCKER_IMAGE}:production"
    docker push "${DOCKER_IMAGE}:production"
    
    log_success "Rollback completed. Please redeploy to apply changes."
}

# Show deployment status
show_status() {
    log_info "Deployment Status"
    echo "=================="
    
    # Local status
    if docker ps | grep -q "${APP_NAME}"; then
        echo -e "Local: ${GREEN}Running${NC}"
        docker ps --filter "name=${APP_NAME}" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
    else
        echo -e "Local: ${RED}Not Running${NC}"
    fi
    
    # Show recent deployments
    echo ""
    echo "Recent Docker Images:"
    docker images "${DOCKER_IMAGE}" --format "table {{.Tag}}\t{{.CreatedAt}}\t{{.Size}}" | head -10
}

# Clean up old images
cleanup() {
    log_info "Cleaning up old images..."
    
    # Remove dangling images
    docker image prune -f
    
    # Keep only last 5 images
    docker images "${DOCKER_IMAGE}" --format "{{.Tag}}" | \
        grep -E "^(backup-|v[0-9])" | \
        sort -r | \
        tail -n +6 | \
        xargs -I {} docker rmi "${DOCKER_IMAGE}:{}" || true
    
    log_success "Cleanup completed"
}

# Main execution
main() {
    echo "======================================"
    echo " RS Helper Deployment Script"
    echo " Environment: ${ENVIRONMENT}"
    echo "======================================"
    echo ""
    
    check_prerequisites
    
    case "${ENVIRONMENT}" in
        local)
            build_app
            run_tests
            deploy_local
            ;;
        staging)
            build_app
            run_tests
            deploy_staging
            ;;
        production|prod)
            build_app
            run_tests
            deploy_production
            ;;
        rollback)
            rollback
            ;;
        status)
            show_status
            ;;
        cleanup)
            cleanup
            ;;
        *)
            log_error "Invalid environment: ${ENVIRONMENT}"
            echo "Usage: $0 [local|staging|production|rollback|status|cleanup]"
            exit 1
            ;;
    esac
    
    log_success "Deployment script completed successfully"
}

# Run main function
main
