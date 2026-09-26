package dev.nova

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.nova.core.NovaService

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Start Nova background foreground service
        val serviceIntent = Intent(this, NovaService::class.java)
        startForegroundService(serviceIntent)

        setContent {
            NovaAndroidApp()
        }
    }
}

enum class NovaScreen(val title: String, val icon: ImageVector) {
    Home("Home", Icons.Default.Home),
    Devices("Devices", Icons.Default.Devices),
    Clipboard("Clipboard", Icons.Default.ContentPaste),
    Notes("Notes", Icons.Default.Notes),
    Files("Files", Icons.Default.Folder),
    Tasks("Tasks", Icons.Default.Task),
    AI("AI", Icons.Default.AutoAwesome),
    Settings("Settings", Icons.Default.Settings)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NovaAndroidApp() {
    var currentScreen by remember { mutableStateOf(NovaScreen.Home) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            text = "NOVA",
                            fontWeight = FontWeight.ExtraBold,
                            color = Color(0xFF6366F1),
                            letterSpacing = 1.sp
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(
                            text = "• " + currentScreen.title,
                            fontSize = 16.sp,
                            color = Color(0xFF94A3B8)
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = Color(0xFF0F172A),
                    titleContentColor = Color.White
                )
            )
        },
        bottomBar = {
            NavigationBar(
                containerColor = Color(0xFF0F172A),
                contentColor = Color(0xFF94A3B8)
            ) {
                listOf(
                    NovaScreen.Home,
                    NovaScreen.Devices,
                    NovaScreen.Clipboard,
                    NovaScreen.Notes,
                    NovaScreen.Files,
                    NovaScreen.Tasks
                ).forEach { screen ->
                    NavigationBarItem(
                        icon = { Icon(screen.icon, contentDescription = screen.title) },
                        label = { Text(screen.title, fontSize = 11.sp) },
                        selected = currentScreen == screen,
                        onClick = { currentScreen = screen },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = Color(0xFF6366F1),
                            selectedTextColor = Color(0xFF6366F1),
                            unselectedIconColor = Color(0xFF64748B),
                            unselectedTextColor = Color(0xFF64748B),
                            indicatorColor = Color(0xFF1E293B)
                        )
                    )
                }
            }
        },
        containerColor = Color(0xFF0A0D14)
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .padding(innerPadding)
                .fillMaxSize()
                .padding(16.dp)
        ) {
            when (currentScreen) {
                NovaScreen.Home -> HomeScreen()
                NovaScreen.Devices -> DevicesScreen()
                NovaScreen.Clipboard -> ClipboardScreen()
                NovaScreen.Notes -> NotesScreen()
                NovaScreen.Files -> FilesScreen()
                NovaScreen.Tasks -> TasksScreen()
                NovaScreen.AI -> Text("Nova AI Orchestrator", color = Color.White)
                NovaScreen.Settings -> Text("Settings & Keystore", color = Color.White)
            }
        }
    }
}

@Composable
fun HomeScreen() {
    Column(modifier = Modifier.fillMaxSize()) {
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF161E2E)),
            shape = RoundedCornerShape(16.dp)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text("Paired PC", color = Color(0xFF94A3B8), fontSize = 13.sp)
                    Text("CONNECTED", color = Color(0xFF10B981), fontSize = 11.sp, fontWeight = FontWeight.Bold)
                }
                Spacer(modifier = Modifier.height(4.dp))
                Text("Prince's Linux PC", color = Color.White, fontSize = 20.sp, fontWeight = FontWeight.Bold)
                Spacer(modifier = Modifier.height(4.dp))
                Text("Direct LAN: 192.168.1.50:53418", color = Color(0xFF64748B), fontSize = 12.sp)
            }
        }

        Spacer(modifier = Modifier.height(20.dp))
        Text("Quick Sync Actions", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 16.sp)
        Spacer(modifier = Modifier.height(12.dp))

        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Button(
                onClick = {},
                modifier = Modifier.weight(1f),
                colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF6366F1)),
                shape = RoundedCornerShape(12.dp)
            ) {
                Text("Send Clipboard", fontSize = 12.sp)
            }
            Button(
                onClick = {},
                modifier = Modifier.weight(1f),
                colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF1E293B)),
                shape = RoundedCornerShape(12.dp)
            ) {
                Text("Send Photo", fontSize = 12.sp, color = Color.White)
            }
        }
    }
}

@Composable
fun DevicesScreen() {
    Column(modifier = Modifier.fillMaxSize()) {
        Text("Paired Linux Machines", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 18.sp)
        Spacer(modifier = Modifier.height(12.dp))
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF161E2E)),
            shape = RoundedCornerShape(12.dp)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("Prince's Linux PC", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 16.sp)
                Text("Fingerprint: 3f8a-9c12-e4b7-10d9", color = Color(0xFF94A3B8), fontSize = 12.sp)
                Spacer(modifier = Modifier.height(8.dp))
                Text("✓ Verified via Noise XX Handshake", color = Color(0xFF10B981), fontSize = 12.sp)
            }
        }
    }
}

@Composable
fun ClipboardScreen() {
    Column(modifier = Modifier.fillMaxSize()) {
        Text("Super Clipboard", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 18.sp)
        Spacer(modifier = Modifier.height(12.dp))
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF161E2E)),
            shape = RoundedCornerShape(12.dp)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("From Prince's Linux PC (Synced)", color = Color(0xFF6366F1), fontSize = 12.sp)
                Spacer(modifier = Modifier.height(4.dp))
                Text("https://github.com/nova-ecosystem/nova", color = Color.White, fontSize = 14.sp)
            }
        }
    }
}

@Composable
fun NotesScreen() {
    Column(modifier = Modifier.fillMaxSize()) {
        Text("Synchronized Notes (CRDT)", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 18.sp)
        Spacer(modifier = Modifier.height(12.dp))
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF161E2E)),
            shape = RoundedCornerShape(12.dp)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("Nova Architectural Specification", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 16.sp)
                Spacer(modifier = Modifier.height(4.dp))
                Text("Peer-to-peer Linux & Android cross-device ecosystem...", color = Color(0xFF94A3B8), fontSize = 13.sp)
            }
        }
    }
}

@Composable
fun FilesScreen() {
    Column(modifier = Modifier.fillMaxSize()) {
        Text("File EasyShare (LAN P2P)", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 18.sp)
        Spacer(modifier = Modifier.height(12.dp))
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF161E2E)),
            shape = RoundedCornerShape(12.dp)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("nova_architecture.pdf", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 15.sp)
                Text("4.2 MB • SHA-256 Verified", color = Color(0xFF10B981), fontSize = 12.sp)
            }
        }
    }
}

@Composable
fun TasksScreen() {
    Column(modifier = Modifier.fillMaxSize()) {
        Text("Task Handoff", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 18.sp)
        Spacer(modifier = Modifier.height(12.dp))
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF161E2E)),
            shape = RoundedCornerShape(12.dp)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("Continue Task: Research WebRTC", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 15.sp)
                Text("From Linux Desktop", color = Color(0xFF6366F1), fontSize = 12.sp)
                Spacer(modifier = Modifier.height(8.dp))
                Button(
                    onClick = {},
                    colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF6366F1)),
                    shape = RoundedCornerShape(8.dp)
                ) {
                    Text("Open on Phone", fontSize = 12.sp)
                }
            }
        }
    }
}
