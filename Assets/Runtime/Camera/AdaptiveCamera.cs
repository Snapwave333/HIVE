using System;
using System.Collections;
using UnityEngine;
using HIVE.Core;

namespace HIVE.Camera
{
    /// <summary>
    /// Four-mode camera system: First-person, Third-person, Isometric, Overview.
    /// Smooth transitions, preserves selection context.
    /// </summary>
    public class AdaptiveCamera : MonoBehaviour
    {
        public static AdaptiveCamera Instance { get; private set; }
        public enum CameraMode
        {
            FirstPerson,
            ThirdPerson,
            Isometric,
            Overview
        }

        [Header("References")]
        public Transform CameraTransform;
        public Transform ThirdPersonAvatar;
        public LayerMask NavigationMask;

        [Header("Mode Settings")]
        [SerializeField] private CameraMode _currentMode = CameraMode.Isometric;
        [SerializeField] private float _transitionDuration = 0.5f;
        [SerializeField] private AnimationCurve _transitionCurve = AnimationCurve.EaseInOut(0, 0, 1, 1);

        [Header("First Person")]
        public float FPS_MoveSpeed = 5f;
        public float FPS_LookSpeed = 2f;
        public float FPS_Gravity = 9.81f;

        [Header("Third Person")]
        public float TPS_Distance = 5f;
        public float TPS_Height = 2f;
        public float TPS_RotationSpeed = 5f;

        [Header("Isometric")]
        public float ISO_Distance = 20f;
        public float ISO_Angle = 30f;
        public float ISO_RotationSpeed = 30f;

        [Header("Overview")]
        public float OVW_Distance = 50f;
        public float OVW_MinDistance = 20f;
        public float OVW_MaxDistance = 100f;

        private CameraMode _targetMode;
        private Coroutine _transitionCoroutine;
        private Vector3 _targetPosition;
        private Quaternion _targetRotation;
        private Vector3 _fpsVelocity;
        private float _fpsPitch;
        private float _isoRotation;
        private float _ovwRotation;
        private Vector3 _ovwCenter = Vector3.zero;

        // Selection context preservation
        public string SelectedAgentId { get; set; }
        public string SelectedProjectId { get; set; }
        public string SelectedTaskId { get; set; }

        public CameraMode CurrentMode => _currentMode;

        private void Awake()
        {
            if (Instance == null) Instance = this;
            if (CameraTransform == null) CameraTransform = UnityEngine.Camera.main?.transform;
            _targetMode = _currentMode;
            ApplyModeImmediate(_currentMode);
        }

        private void Update()
        {
            HandleInput();
            UpdateCurrentMode();
        }

        private void HandleInput()
        {
            // Mode switching
            if (Input.GetKeyDown(KeyCode.Alpha1)) SetMode(CameraMode.FirstPerson);
            if (Input.GetKeyDown(KeyCode.Alpha2)) SetMode(CameraMode.ThirdPerson);
            if (Input.GetKeyDown(KeyCode.Alpha3)) SetMode(CameraMode.Isometric);
            if (Input.GetKeyDown(KeyCode.Alpha4)) SetMode(CameraMode.Overview);

            // Mode-specific controls
            switch (_currentMode)
            {
                case CameraMode.FirstPerson: HandleFPSInput(); break;
                case CameraMode.ThirdPerson: HandleTPSInput(); break;
                case CameraMode.Isometric: HandleISOInput(); break;
                case CameraMode.Overview: HandleOVWInput(); break;
            }

            // Universal: focus on selection
            if (Input.GetKeyDown(KeyCode.F) && !string.IsNullOrEmpty(SelectedAgentId))
                FocusOnAgent(SelectedAgentId);
        }

        public void SetMode(CameraMode mode, bool instant = false)
        {
            if (mode == _currentMode && !instant) return;
            _targetMode = mode;

            if (instant || _transitionDuration <= 0)
            {
                _currentMode = mode;
                ApplyModeImmediate(mode);
            }
            else
            {
                if (_transitionCoroutine != null) StopCoroutine(_transitionCoroutine);
                _transitionCoroutine = StartCoroutine(TransitionToMode(mode));
            }
        }

        private IEnumerator TransitionToMode(CameraMode targetMode)
        {
            var startPos = CameraTransform.position;
            var startRot = CameraTransform.rotation;
            var elapsed = 0f;

            // Calculate target transform for new mode
            CalculateTargetTransform(targetMode, out var endPos, out var endRot);

            while (elapsed < _transitionDuration)
            {
                elapsed += Time.deltaTime;
                var t = _transitionCurve.Evaluate(elapsed / _transitionDuration);
                CameraTransform.position = Vector3.Lerp(startPos, endPos, t);
                CameraTransform.rotation = Quaternion.Slerp(startRot, endRot, t);
                yield return null;
            }

            CameraTransform.position = endPos;
            CameraTransform.rotation = endRot;
            _currentMode = targetMode;
        }

        private void CalculateTargetTransform(CameraMode mode, out Vector3 pos, out Quaternion rot)
        {
            switch (mode)
            {
                case CameraMode.FirstPerson:
                    pos = CameraTransform.position;
                    rot = CameraTransform.rotation;
                    break;
                case CameraMode.ThirdPerson:
                    if (ThirdPersonAvatar != null)
                    {
                        var dir = Quaternion.Euler(0, _ovwRotation, 0) * Vector3.back;
                        pos = ThirdPersonAvatar.position + Vector3.up * TPS_Height + dir * TPS_Distance;
                        rot = Quaternion.LookRotation(ThirdPersonAvatar.position - pos);
                    }
                    else goto case CameraMode.Isometric;
                    break;
                case CameraMode.Isometric:
                    var isoDir = Quaternion.Euler(0, _isoRotation, 0) * Quaternion.Euler(-ISO_Angle, 0, 0) * Vector3.forward;
                    pos = _ovwCenter + isoDir * ISO_Distance;
                    rot = Quaternion.LookRotation(_ovwCenter - pos);
                    break;
                case CameraMode.Overview:
                    var ovwDir = Quaternion.Euler(0, _ovwRotation, 0) * Quaternion.Euler(-45, 0, 0) * Vector3.forward;
                    pos = _ovwCenter + ovwDir * OVW_Distance;
                    rot = Quaternion.LookRotation(_ovwCenter - pos);
                    break;
                default:
                    pos = CameraTransform.position;
                    rot = CameraTransform.rotation;
                    break;
            }
        }

        private void ApplyModeImmediate(CameraMode mode)
        {
            CalculateTargetTransform(mode, out var pos, out var rot);
            CameraTransform.position = pos;
            CameraTransform.rotation = rot;
            _currentMode = mode;
        }

        private void UpdateCurrentMode()
        {
            // Continuous updates for modes that need it
            switch (_currentMode)
            {
                case CameraMode.ThirdPerson:
                    if (ThirdPersonAvatar != null)
                    {
                        var dir = Quaternion.Euler(0, _ovwRotation, 0) * Vector3.back;
                        var targetPos = ThirdPersonAvatar.position + Vector3.up * TPS_Height + dir * TPS_Distance;
                        CameraTransform.position = Vector3.Lerp(CameraTransform.position, targetPos, Time.deltaTime * 10f);
                        CameraTransform.LookAt(ThirdPersonAvatar.position + Vector3.up * 1.5f);
                    }
                    break;
            }
        }

        // ========== Mode Input Handlers ==========

        private void HandleFPSInput()
        {
            // Mouse look
            var mouseX = Input.GetAxis("Mouse X") * FPS_LookSpeed;
            var mouseY = Input.GetAxis("Mouse Y") * FPS_LookSpeed;
            _fpsPitch = Mathf.Clamp(_fpsPitch - mouseY, -85f, 85f);
            CameraTransform.localRotation = Quaternion.Euler(_fpsPitch, 0, 0);
            transform.Rotate(Vector3.up * mouseX);

            // Movement
            var input = new Vector3(Input.GetAxis("Horizontal"), 0, Input.GetAxis("Vertical"));
            var move = transform.TransformDirection(input) * FPS_MoveSpeed;
            _fpsVelocity = Vector3.Lerp(_fpsVelocity, move, Time.deltaTime * 10f);
            transform.position += _fpsVelocity * Time.deltaTime;
        }

        private void HandleTPSInput()
        {
            if (ThirdPersonAvatar == null) return;

            // Orbit
            _ovwRotation += Input.GetAxis("Mouse X") * TPS_RotationSpeed * Time.deltaTime;
            TPS_Distance = Mathf.Clamp(TPS_Distance - Input.GetAxis("Mouse ScrollWheel") * 5f, 2f, 15f);

            // Avatar movement
            var input = new Vector3(Input.GetAxis("Horizontal"), 0, Input.GetAxis("Vertical"));
            if (input.magnitude > 0.1f)
            {
                var moveDir = Quaternion.Euler(0, _ovwRotation, 0) * input.normalized;
                ThirdPersonAvatar.position += moveDir * FPS_MoveSpeed * Time.deltaTime;
                ThirdPersonAvatar.forward = Vector3.Lerp(ThirdPersonAvatar.forward, moveDir, Time.deltaTime * 10f);
            }
        }

        private void HandleISOInput()
        {
            _isoRotation += Input.GetAxis("Horizontal") * ISO_RotationSpeed * Time.deltaTime;
            ISO_Distance = Mathf.Clamp(ISO_Distance - Input.GetAxis("Mouse ScrollWheel") * 10f, 10f, 50f);

            if (Input.GetMouseButton(2)) // Middle mouse pan
            {
                var pan = new Vector3(-Input.GetAxis("Mouse X"), 0, -Input.GetAxis("Mouse Y")) * ISO_Distance * 0.01f;
                _ovwCenter += CameraTransform.TransformDirection(pan);
            }
        }

        private void HandleOVWInput()
        {
            _ovwRotation += Input.GetAxis("Horizontal") * 20f * Time.deltaTime;
            OVW_Distance = Mathf.Clamp(OVW_Distance - Input.GetAxis("Mouse ScrollWheel") * 20f, OVW_MinDistance, OVW_MaxDistance);

            if (Input.GetMouseButton(2))
            {
                var pan = new Vector3(-Input.GetAxis("Mouse X"), 0, -Input.GetAxis("Mouse Y")) * OVW_Distance * 0.01f;
                _ovwCenter += CameraTransform.TransformDirection(pan);
            }
        }

        public void FocusOnAgent(string agentId)
        {
            // Find agent workspace and focus camera
            var workspace = GameObject.Find($"Workspace_{agentId}");
            if (workspace != null)
            {
                _ovwCenter = workspace.transform.position;
                SetMode(CameraMode.Isometric, true);
            }
        }

        public void FocusOnPosition(Vector3 position)
        {
            _ovwCenter = position;
        }

        public void SetOverviewCenter(Vector3 center)
        {
            _ovwCenter = center;
        }
    }
}