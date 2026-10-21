export interface IconState {
  icon: string;
  name: string;
  text: string;
}

export const states: IconState[] = [
  { icon: 'icon-idle.png', name: 'Idle — emerald', text: 'Everything installed is current.' },
  { icon: 'icon-updates.png', name: 'Updates — amber', text: 'One or more packages are behind.' },
  {
    icon: 'icon-error.png',
    name: 'Error — red',
    text: 'A check, a source or a registry failed, and the goblin sweats. A failure never renders as “everything is fine”.',
  },
  {
    icon: 'icon-busy.png',
    name: 'Busy — sky blue',
    text: 'A check or an update is running: the goblin fills like a glass and twitches its ears.',
  },
];
