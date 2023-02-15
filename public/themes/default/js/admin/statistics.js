// Jexactyl Software. (https://jexactyl.com)
// Green: #189a1c
// Gray: hsl(211, 22%, 21%)

console.log(Pterodactyl);

const suspended = Pterodactyl.suspended;
const active = Pterodactyl.servers.length - Pterodactyl.suspended;
const freeDisk = Pterodactyl.diskTotal - Pterodactyl.diskUsed;
const freeMemory = Pterodactyl.memoryTotal - Pterodactyl.memoryUsed;

const diskChart = new Chart($("#disk_chart"), {
    type: "pie",
    data: {
        labels: ["Disco Livre", "Disco Usado"],
        datasets: [{
            backgroundColor: ["#189a1c", "hsl(211, 22%, 21%)"],
            data: [freeDisk, Pterodactyl.diskUsed]
        }]
    }
});

const ramChart = new Chart($("#ram_chart"), {
    type: "pie",
    data: {
        labels: ["RAM Livre", "RAM Usada"],
        datasets: [{
            backgroundColor: ["#189a1c", "hsl(211, 22%, 21%)"],
            data: [freeMemory, Pterodactyl.memoryUsed]
        }]
    }
});

const serversChart = new Chart($("#servers_chart"), {
    type: "pie",
    data: {
        labels: ["Servidores Ativos", "Servidores Suspensos"],
        datasets: [{
            backgroundColor: ["#189a1c", "hsl(211, 22%, 21%)"],
            data: [active, suspended]
        }]
    }
});
