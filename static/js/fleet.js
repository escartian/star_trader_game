// Fleet-related functions
async function showFleet(fleetNumber) {
    try {
        const response = await fetch(`/fleet/${playerName}/${fleetNumber}`);
        const fleetData = await response.json();
        
        if (!fleetData) {
            document.getElementById('result').innerHTML += `<p>Fleet not found</p>`;
            return;
        }

        const container = document.createElement('div');
        container.className = 'fleet-container';
        container.innerHTML = `
            <div class="info-panel">
                <h2>Fleet Information</h2>
                <h3>${fleetData.name}</h3>
                <p>Owner: ${fleetData.owner_id}</p>
                <p>Position: (${fleetData.position.x}, ${fleetData.position.y}, ${fleetData.position.z})</p>
                <h3>Ships</h3>
                <table class="fleet-table">
                    <tr>
                        <th>Name</th>
                        <th>Type</th>
                        <th>Size</th>
                        <th>Health</th>
                        <th>Shields</th>
                        <th>Cargo</th>
                    </tr>
                    ${fleetData.ships.map(ship => `
                        <tr>
                            <td>${ship.name}</td>
                            <td>${ship.specialization}</td>
                            <td>${ship.size}</td>
                            <td>${ship.hp}</td>
                            <td>${ship.shields.strength}</td>
                            <td>${ship.get_current_cargo()} / ${ship.get_cargo_capacity()}</td>
                        </tr>
                    `).join('')}
                </table>
            </div>
        `;
        document.getElementById('result').appendChild(container);
    } catch (error) {
        console.error("Error fetching fleet data:", error);
        document.getElementById('result').innerHTML += `<p>Error: Unable to load fleet data</p>`;
    }
}

async function showFleets() {
    try {
        const response = await fetch(`/fleets/${playerName}`);
        const fleets = await response.json();
        
        const container = document.createElement('div');
        container.className = 'fleet-container';
        container.innerHTML = `
            <div class="info-panel">
                <h2>Your Fleets</h2>
                <table class="fleet-table">
                    <tr>
                        <th>Fleet Name</th>
                        <th>Position</th>
                        <th>Ships</th>
                        <th>Actions</th>
                    </tr>
                    ${fleets.map(fleet => {
                        // Extract fleet number from the fleet name (e.g., "Fleet_Igor_1" -> 1)
                        const fleetNumber = fleet.name.split('_').pop();
                        return `
                            <tr>
                                <td>${fleet.name}</td>
                                <td>(${fleet.position.x}, ${fleet.position.y}, ${fleet.position.z})</td>
                                <td>${fleet.ships.length}</td>
                                <td>
                                    <button onclick="showFleet('${fleetNumber}')">View Details</button>
                                </td>
                            </tr>
                        `;
                    }).join('')}
                </table>
            </div>
        `;
        document.getElementById('result').appendChild(container);
    } catch (error) {
        console.error("Error fetching fleets:", error);
        document.getElementById('result').innerHTML += `<p>Error: Unable to load fleets</p>`;
    }
} 