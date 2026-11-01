using System;
using UnityEngine;
using UnityEngine.UIElements;
using HIVE.Core;

namespace HIVE.UI
{
    /// <summary>
    /// Manages three density modes: Dense, Comfortable, Focus (ADHD-friendly).
    /// </summary>
    public class DensityController : MonoBehaviour
    {
        public enum DensityMode { Dense, Comfortable, Focus }

        [Header("Settings")]
        public DensityMode CurrentMode = DensityMode.Comfortable;
        public KeyCode ToggleKey = KeyCode.F1;

        [Header("Mode Config")]
        public DensityConfig DenseConfig = new DensityConfig
        {
            PanelSpacing = 4,
            FontSize = 11,
            AnimationSpeed = 1f,
            ShowBackgroundActivity = true,
            ShowNonCriticalNotifications = true,
            MinimizePanels = false
        };

        public DensityConfig ComfortableConfig = new DensityConfig
        {
            PanelSpacing = 8,
            FontSize = 13,
            AnimationSpeed = 1f,
            ShowBackgroundActivity = true,
            ShowNonCriticalNotifications = false,
            MinimizePanels = false
        };

        public DensityConfig FocusConfig = new DensityConfig
        {
            PanelSpacing = 12,
            FontSize = 15,
            AnimationSpeed = 0.3f,
            ShowBackgroundActivity = false,
            ShowNonCriticalNotifications = false,
            MinimizePanels = true
        };

        private DensityConfig _activeConfig;
        private VisualElement _rootStyle;

        public event Action<DensityMode> OnModeChanged;

        private void Awake()
        {
            _rootStyle = GetComponent<UIDocument>()?.rootVisualElement;
            ApplyMode(CurrentMode);
        }

        private void Update()
        {
            if (Input.GetKeyDown(ToggleKey))
                CycleMode();
        }

        public void SetMode(DensityMode mode)
        {
            CurrentMode = mode;
            ApplyMode(mode);
            OnModeChanged?.Invoke(mode);
        }

        public void CycleMode()
        {
            var modes = Enum.GetValues(typeof(DensityMode));
            var next = (int)CurrentMode + 1;
            if (next >= modes.Length) next = 0;
            SetMode((DensityMode)next);
        }

        private void ApplyMode(DensityMode mode)
        {
            _activeConfig = mode switch
            {
                DensityMode.Dense => DenseConfig,
                DensityMode.Comfortable => ComfortableConfig,
                DensityMode.Focus => FocusConfig,
                _ => ComfortableConfig
            };

            // Apply to UI Toolkit stylesheet
            if (_rootStyle != null)
            {
                _rootStyle.style.fontSize = _activeConfig.FontSize;
                _rootStyle.EnableInClassList("dense", mode == DensityMode.Dense);
                _rootStyle.EnableInClassList("comfortable", mode == DensityMode.Comfortable);
                _rootStyle.EnableInClassList("focus", mode == DensityMode.Focus);
            }

            // Notify other systems
            MessageBus.Publish(new DensityChangedMessage { Mode = mode, Config = _activeConfig });

            // Focus mode: reduce animation globally
            if (mode == DensityMode.Focus)
            {
                Time.timeScale = 1f; // Keep normal time
                // Could set global animation multiplier here
            }
        }

        public DensityConfig GetConfig() => _activeConfig;

        [Serializable]
        public class DensityConfig
        {
            public int PanelSpacing;
            public int FontSize;
            public float AnimationSpeed;
            public bool ShowBackgroundActivity;
            public bool ShowNonCriticalNotifications;
            public bool MinimizePanels;
        }
    }

    public class DensityChangedMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public DensityController.DensityMode Mode { get; set; }
        public DensityController.DensityConfig Config { get; set; }
    }
}