import 'package:flutter/material.dart';

typedef LocalLogin = Future<void> Function(String username, String password);

class MonolithLoginView extends StatefulWidget {
  const MonolithLoginView({
    super.key,
    required this.backendUrl,
    required this.onSaveBackendUrl,
    required this.onLogin,
    required this.onOffline,
    this.busy = false,
    this.notice,
  });

  final String backendUrl;
  final Future<void> Function(String) onSaveBackendUrl;
  final LocalLogin onLogin;
  final VoidCallback onOffline;
  final bool busy;
  final String? notice;

  @override
  State<MonolithLoginView> createState() => _MonolithLoginViewState();
}

class _MonolithLoginViewState extends State<MonolithLoginView> {
  late final TextEditingController _backend = TextEditingController(
    text: widget.backendUrl,
  );
  final TextEditingController _username = TextEditingController();
  final TextEditingController _password = TextEditingController();

  @override
  void didUpdateWidget(MonolithLoginView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.backendUrl != oldWidget.backendUrl) {
      _backend.text = widget.backendUrl;
    }
  }

  @override
  void dispose() {
    _backend.dispose();
    _username.dispose();
    _password.dispose();
    super.dispose();
  }

  Future<void> _login() async {
    final password = _password.text;
    try {
      await widget.onLogin(_username.text, password);
    } finally {
      _password.clear();
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    body: SafeArea(
      child: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 440),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  'MONOLITH',
                  style: Theme.of(context).textTheme.headlineLarge,
                ),
                const SizedBox(height: 12),
                const Text(
                  'Connexion au catalogue · accès hors ligne disponible',
                ),
                if (widget.busy) const LinearProgressIndicator(),
                if (widget.notice != null) ...[
                  const SizedBox(height: 12),
                  Text(
                    widget.notice!,
                    style: const TextStyle(color: Colors.orangeAccent),
                  ),
                ],
                const SizedBox(height: 24),
                TextField(
                  key: const Key('backend-url'),
                  controller: _backend,
                  decoration: const InputDecoration(
                    labelText: 'URL du serveur',
                  ),
                ),
                const SizedBox(height: 8),
                OutlinedButton(
                  onPressed: widget.busy
                      ? null
                      : () => widget.onSaveBackendUrl(_backend.text),
                  child: const Text('Enregistrer le serveur'),
                ),
                const SizedBox(height: 24),
                TextField(
                  key: const Key('username'),
                  controller: _username,
                  decoration: const InputDecoration(
                    labelText: 'Nom d’utilisateur',
                  ),
                ),
                const SizedBox(height: 8),
                TextField(
                  key: const Key('password'),
                  controller: _password,
                  obscureText: true,
                  decoration: const InputDecoration(labelText: 'Mot de passe'),
                  onSubmitted: (_) {
                    if (!widget.busy) _login();
                  },
                ),
                const SizedBox(height: 16),
                FilledButton(
                  onPressed: widget.busy ? null : _login,
                  child: const Text('Connexion locale'),
                ),
                TextButton(
                  onPressed: widget.busy ? null : widget.onOffline,
                  child: const Text('Continuer hors ligne'),
                ),
              ],
            ),
          ),
        ),
      ),
    ),
  );
}
