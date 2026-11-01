using UnityEngine;

namespace HIVE.Core
{
    /// <summary>
    /// Safe input wrapper that guards input calls to prevent
    /// InvalidOperationExceptions during platform transitions.
    /// </summary>
    public static class HiveInput
    {
        public static bool GetKeyDown(KeyCode key)
        {
            try
            {
                return Input.GetKeyDown(key);
            }
            catch
            {
                return false;
            }
        }

        public static float GetAxis(string axisName)
        {
            try
            {
                return Input.GetAxis(axisName);
            }
            catch
            {
                return 0f;
            }
        }

        public static bool GetMouseButton(int button)
        {
            try
            {
                return Input.GetMouseButton(button);
            }
            catch
            {
                return false;
            }
        }
    }
}
