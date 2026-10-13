import 'package:flutter/material.dart';

/// The Rust service owns validation, persistence, reset and RetroArch export.
class ControllerSettings extends StatefulWidget {
  const ControllerSettings({
    super.key,
    required this.initialConfig,
    required this.detected,
    required this.onSave,
    required this.onReset,
    required this.onReload,
  });
  final String initialConfig;
  final List<String> detected;
  final Future<void> Function(String) onSave;
  final Future<void> Function(int, String) onReset;
  final Future<String> Function() onReload;

  @override
  State<ControllerSettings> createState() => _ControllerSettingsState();
}

class _ControllerSettingsState extends State<ControllerSettings> {
  late final TextEditingController _config = TextEditingController(
    text: widget.initialConfig,
  );
  final TextEditingController _system = TextEditingController(text: '11');
  final TextEditingController _device = TextEditingController();
  String? _error;
  bool _busy = false;

  @override
  void dispose() {
    _config.dispose();
    _system.dispose();
    _device.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await widget.onSave(_config.text);
      if (mounted) Navigator.of(context).pop(true);
    } catch (error) {
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _reset() async {
    final id = int.tryParse(_system.text);
    if (id == null || id <= 0 || _device.text.trim().isEmpty) {
      setState(
        () => _error =
            'Indiquez la console et le device_id du profil à réinitialiser.',
      );
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await widget.onReset(id, _device.text.trim());
      final updated = await widget.onReload();
      if (mounted) setState(() => _config.text = updated);
    } catch (error) {
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Manettes par console'),
    content: SizedBox(
      width: 660,
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('Manettes détectées sur ce client :'),
            for (final device in widget.detected) Text(device),
            if (widget.detected.isEmpty) const Text('Aucune manette détectée'),
            const SizedBox(height: 8),
            const Text(
              'Un profil actif par console : system_id, device_id, [profile.buttons]. '
              'RetroArch reçoit une surcouche par console ; PCSX2/Xenia ne sont pas encore pris en charge.',
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _config,
              minLines: 8,
              maxLines: 14,
              decoration: const InputDecoration(
                labelText: 'controllers.toml',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _system,
                    keyboardType: TextInputType.number,
                    decoration: const InputDecoration(labelText: 'system_id'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: TextField(
                    controller: _device,
                    decoration: const InputDecoration(labelText: 'device_id'),
                  ),
                ),
              ],
            ),
            TextButton(
              onPressed: _busy ? null : _reset,
              child: const Text('RAZ valeurs par défaut'),
            ),
            if (_error != null)
              SelectableText(
                _error!,
                style: const TextStyle(color: Colors.orangeAccent),
              ),
          ],
        ),
      ),
    ),
    actions: [
      TextButton(
        onPressed: _busy ? null : () => Navigator.pop(context),
        child: const Text('Annuler'),
      ),
      FilledButton(
        onPressed: _busy ? null : _save,
        child: const Text('Enregistrer'),
      ),
    ],
  );
}
