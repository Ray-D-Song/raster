import type { IconifyIcon } from "raster-js/components";

import addIcon from "@iconify-icons/material-symbols/add";
import arrowDownwardIcon from "@iconify-icons/material-symbols/arrow-downward";
import arrowUpwardIcon from "@iconify-icons/material-symbols/arrow-upward";
import calendarMonthIcon from "@iconify-icons/material-symbols/calendar-month";
import cameraIcon from "@iconify-icons/material-symbols/photo-camera";
import contentCopyIcon from "@iconify-icons/material-symbols/content-copy";
import imageIcon from "@iconify-icons/material-symbols/image";
import checkIcon from "@iconify-icons/material-symbols/check";
import chevronLeftIcon from "@iconify-icons/material-symbols/chevron-left";
import chevronRightIcon from "@iconify-icons/material-symbols/chevron-right";
import darkModeIcon from "@iconify-icons/material-symbols/dark-mode";
import dashboardIcon from "@iconify-icons/material-symbols/dashboard";
import deleteIcon from "@iconify-icons/material-symbols/delete";
import horizontalRuleIcon from "@iconify-icons/material-symbols/horizontal-rule";
import infoIcon from "@iconify-icons/material-symbols/info";
import notificationsIcon from "@iconify-icons/material-symbols/notifications";
import openInNewIcon from "@iconify-icons/material-symbols/open-in-new";
import pieChartIcon from "@iconify-icons/material-symbols/pie-chart";
import settingsIcon from "@iconify-icons/material-symbols/settings";
import starIcon from "@iconify-icons/material-symbols/star";
import thumbDownIcon from "@iconify-icons/material-symbols/thumb-down";
import thumbUpIcon from "@iconify-icons/material-symbols/thumb-up";
import warningIcon from "@iconify-icons/material-symbols/warning";

export const appIcons = {
  add: addIcon,
  arrowDown: arrowDownwardIcon,
  arrowUp: arrowUpwardIcon,
  calendar: calendarMonthIcon,
  camera: cameraIcon,
  copy: contentCopyIcon,
  image: imageIcon,
  check: checkIcon,
  chevronLeft: chevronLeftIcon,
  chevronRight: chevronRightIcon,
  darkMode: darkModeIcon,
  dashboard: dashboardIcon,
  delete: deleteIcon,
  horizontalRule: horizontalRuleIcon,
  info: infoIcon,
  notifications: notificationsIcon,
  openInNew: openInNewIcon,
  pieChart: pieChartIcon,
  settings: settingsIcon,
  star: starIcon,
  thumbDown: thumbDownIcon,
  thumbUp: thumbUpIcon,
  warning: warningIcon,
} satisfies Record<string, IconifyIcon>;