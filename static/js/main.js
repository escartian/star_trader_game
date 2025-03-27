// Global variables
let playerName = '';

// Initialize the application
document.addEventListener('DOMContentLoaded', () => {
    // Get player name from the template
    playerName = document.getElementById('player-name').textContent;
    
    // Add event listeners
    document.getElementById('show-galaxy-map').addEventListener('click', showGalaxyMap);
    document.getElementById('show-fleets').addEventListener('click', showFleets);
});

// Common utility functions
function showNotification(message, type = 'info') {
    const feedbackDiv = document.createElement('div');
    feedbackDiv.className = `info-panel notification ${type}`;
    feedbackDiv.innerHTML = `
        ${message}
        <span class="close-notification" onclick="this.parentElement.remove()">&times;</span>
    `;
    document.getElementById('result').insertBefore(feedbackDiv, document.querySelector('.market-container'));
    
    // Auto-dismiss after 5 seconds
    setTimeout(() => {
        if (feedbackDiv.parentNode) {
            feedbackDiv.remove();
        }
    }, 5000);
}

// Clear the result container
function clearResult() {
    document.getElementById('result').innerHTML = '';
} 